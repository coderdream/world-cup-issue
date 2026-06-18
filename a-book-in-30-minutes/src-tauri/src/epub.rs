use crate::commands::{command_error, CommandError};
use crate::models::{EpubBook, EpubChapter, EpubChapterSummary, EpubOverview};
use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

pub fn read_epub(path: &Path) -> Result<EpubBook, CommandError> {
    let file = File::open(path).map_err(|error| command_error(format!("读取 EPUB 失败：{error}")))?;
    let mut archive = ZipArchive::new(file).map_err(|error| command_error(format!("打开 EPUB 压缩包失败：{error}")))?;
    let container = read_zip_text(&mut archive, "META-INF/container.xml")?;
    let opf_path = capture_attr(&container, "rootfile", "full-path").ok_or_else(|| command_error("EPUB 缺少 OPF 路径。"))?;
    let opf = read_zip_text(&mut archive, &opf_path)?;
    let base = Path::new(&opf_path).parent().map(Path::to_path_buf).unwrap_or_default();

    let manifest = parse_manifest(&opf);
    let spine = parse_spine(&opf);
    let toc = read_toc_titles(&mut archive, &base, &manifest).unwrap_or_default();

    let title = first_dc_value(&opf, "title").unwrap_or_else(|| fallback_title(path));
    let creator = first_dc_value(&opf, "creator").unwrap_or_default();
    let publisher = first_dc_value(&opf, "publisher").unwrap_or_default();
    let language = first_dc_value(&opf, "language").unwrap_or_default();

    let mut chapters = Vec::new();
    let mut chapter_summaries = Vec::new();
    let mut total_chars = 0usize;

    for idref in spine {
        let Some(href) = manifest.get(&idref) else {
            continue;
        };
        if !is_html_href(href) {
            continue;
        }
        let full_path = join_epub_path(&base, href);
        let raw = match read_zip_text(&mut archive, &full_path) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let text = clean_html_text(&raw);
        let chars = count_han_chars(&text);
        if chars < 20 {
            continue;
        }
        let title = toc
            .get(&normalize_epub_path(&full_path))
            .cloned()
            .or_else(|| extract_title_from_html(&raw))
            .unwrap_or_else(|| title_from_href(href));
        total_chars += chars;
        chapter_summaries.push(EpubChapterSummary {
            title: title.clone(),
            chars,
        });
        chapters.push(EpubChapter { title, text });
    }

    Ok(EpubBook {
        overview: EpubOverview {
            title,
            creator,
            publisher,
            language,
            total_chars,
            chapters: chapter_summaries,
        },
        chapters,
    })
}

fn read_zip_text<R: Read + std::io::Seek>(archive: &mut ZipArchive<R>, path: &str) -> Result<String, CommandError> {
    let mut file = archive
        .by_name(path)
        .map_err(|error| command_error(format!("读取 EPUB 文件 {path} 失败：{error}")))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|error| command_error(format!("读取 EPUB 文件内容失败：{error}")))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn parse_manifest(opf: &str) -> HashMap<String, String> {
    let mut manifest = HashMap::new();
    let item_re = Regex::new(r#"(?is)<item\b[^>]*>"#).expect("valid regex");
    for item in item_re.find_iter(opf) {
        let tag = item.as_str();
        let Some(id) = attr_value(tag, "id") else {
            continue;
        };
        let Some(href) = attr_value(tag, "href") else {
            continue;
        };
        manifest.insert(id, decode_xml_entities(&href));
    }
    manifest
}

fn parse_spine(opf: &str) -> Vec<String> {
    let item_ref_re = Regex::new(r#"(?is)<itemref\b[^>]*>"#).expect("valid regex");
    item_ref_re
        .find_iter(opf)
        .filter_map(|item| attr_value(item.as_str(), "idref"))
        .collect()
}

fn read_toc_titles<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    base: &Path,
    manifest: &HashMap<String, String>,
) -> Result<HashMap<String, String>, CommandError> {
    let toc_href = manifest
        .values()
        .find(|href| href.to_ascii_lowercase().ends_with(".ncx"))
        .cloned()
        .unwrap_or_else(|| "toc.ncx".to_string());
    let toc_path = join_epub_path(base, &toc_href);
    let toc = read_zip_text(archive, &toc_path)?;
    let nav_re = Regex::new(r#"(?is)<navPoint\b.*?</navPoint>"#).expect("valid regex");
    let text_re = Regex::new(r#"(?is)<text[^>]*>(.*?)</text>"#).expect("valid regex");
    let content_re = Regex::new(r#"(?is)<content\b[^>]*\bsrc\s*=\s*["']([^"']+)["'][^>]*/?>"#).expect("valid regex");
    let mut titles = HashMap::new();
    for nav in nav_re.find_iter(&toc) {
        let block = nav.as_str();
        let title = text_re
            .captures(block)
            .and_then(|captures| captures.get(1))
            .map(|value| clean_inline_text(value.as_str()));
        let src = content_re
            .captures(block)
            .and_then(|captures| captures.get(1))
            .map(|value| value.as_str().split('#').next().unwrap_or("").to_string());
        if let (Some(title), Some(src)) = (title, src) {
            let full = join_epub_path(base, &src);
            titles.insert(normalize_epub_path(&full), title);
        }
    }
    Ok(titles)
}

fn first_dc_value(opf: &str, tag: &str) -> Option<String> {
    let pattern = format!(r#"(?is)<(?:dc:)?{}\b[^>]*>(.*?)</(?:dc:)?{}>"#, regex::escape(tag), regex::escape(tag));
    let re = Regex::new(&pattern).ok()?;
    re.captures(opf)
        .and_then(|captures| captures.get(1))
        .map(|value| clean_inline_text(value.as_str()))
        .filter(|value| !value.is_empty())
}

fn capture_attr(xml: &str, tag: &str, attr: &str) -> Option<String> {
    let tag_re = Regex::new(&format!(r#"(?is)<{}\b[^>]*>"#, regex::escape(tag))).ok()?;
    let value = tag_re.find_iter(xml).find_map(|matched| attr_value(matched.as_str(), attr));
    value
}

fn attr_value(tag: &str, attr: &str) -> Option<String> {
    let attr_re = Regex::new(&format!(r#"(?is)\b{}\s*=\s*["']([^"']+)["']"#, regex::escape(attr))).ok()?;
    attr_re
        .captures(tag)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str().to_string())
}

fn clean_html_text(raw: &str) -> String {
    let script_re = Regex::new(r#"(?is)<(script|style)\b.*?</\1>"#).expect("valid regex");
    let block_re = Regex::new(r#"(?is)</?(p|div|h[1-6]|br|li|section|article|body|title)\b[^>]*>"#).expect("valid regex");
    let tag_re = Regex::new(r#"(?is)<[^>]+>"#).expect("valid regex");
    let whitespace_re = Regex::new(r#"[ \t\r\f\v]+"#).expect("valid regex");
    let blank_re = Regex::new(r#"\n\s*\n+"#).expect("valid regex");
    let without_scripts = script_re.replace_all(raw, "");
    let with_breaks = block_re.replace_all(&without_scripts, "\n");
    let without_tags = tag_re.replace_all(&with_breaks, "");
    let decoded = decode_xml_entities(&without_tags);
    let compact = whitespace_re.replace_all(&decoded, " ");
    blank_re.replace_all(compact.trim(), "\n").trim().to_string()
}

fn extract_title_from_html(raw: &str) -> Option<String> {
    let re = Regex::new(r#"(?is)<title[^>]*>(.*?)</title>"#).ok()?;
    re.captures(raw)
        .and_then(|captures| captures.get(1))
        .map(|value| clean_inline_text(value.as_str()))
        .filter(|value| !value.is_empty())
}

fn clean_inline_text(raw: &str) -> String {
    let tag_re = Regex::new(r#"(?is)<[^>]+>"#).expect("valid regex");
    let whitespace_re = Regex::new(r#"\s+"#).expect("valid regex");
    let decoded = decode_xml_entities(&tag_re.replace_all(raw, ""));
    whitespace_re.replace_all(decoded.trim(), " ").trim().to_string()
}

fn decode_xml_entities(value: &str) -> String {
    value
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#160;", " ")
}

fn join_epub_path(base: &Path, href: &str) -> String {
    let href = href.split('#').next().unwrap_or(href);
    let mut parts: Vec<String> = Vec::new();
    for component in base.join(PathBuf::from(href)).components() {
        let value = component.as_os_str().to_string_lossy();
        match value.as_ref() {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(value.replace('\\', "/")),
        }
    }
    parts.join("/")
}

fn normalize_epub_path(path: &str) -> String {
    join_epub_path(Path::new(""), path)
}

fn title_from_href(href: &str) -> String {
    Path::new(href)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("未命名章节")
        .to_string()
}

fn fallback_title(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("未命名图书")
        .to_string()
}

fn is_html_href(href: &str) -> bool {
    let lower = href.to_ascii_lowercase();
    lower.ends_with(".xhtml") || lower.ends_with(".html") || lower.ends_with(".htm")
}

pub fn count_han_chars(value: &str) -> usize {
    value.chars().filter(|ch| ('\u{4e00}'..='\u{9fff}').contains(ch)).count()
}

pub fn truncate_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

#[allow(dead_code)]
fn _read_zip_text_from_bytes(bytes: Vec<u8>, path: &str) -> Result<String, CommandError> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|error| command_error(format!("打开 EPUB 压缩包失败：{error}")))?;
    read_zip_text(&mut archive, path)
}
