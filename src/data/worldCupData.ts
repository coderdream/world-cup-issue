import type { AppSettings, BracketNode, LicenseState, Match, Team } from "@/types";

export const APP_VERSION = "0.1.11";
export const TOTAL_MATCHES = 104;
export const MATCH_STATS = {
  all: 104,
  live: 1,
  finished: 18,
  scheduled: 85
};

export const defaultSettings: AppSettings = {
  spoilerMode: false,
  scorebarEnabled: false,
  launchOnBoot: false,
  notificationsEnabled: true,
  reminderMinutes: 15,
  footballDataToken: "",
  aiProvider: "OpenAI Compatible",
  aiApiKey: "",
  aiBaseUrl: "http://81.68.73.15:3000/openai/v1",
  aiModel: "gpt-5.5",
  aiProfileName: "杯况 CupWatch"
};

export const defaultLicense: LicenseState = {
  status: "trial",
  remainingDays: 30,
  expiresAt: "2026-07-16"
};

const teamSeed = [
  ["mex", "MEX", "墨西哥", "Mexico", "A", "🇲🇽", 1950],
  ["rsa", "RSA", "南非", "South Africa", "A", "🇿🇦", 1690],
  ["kor", "KOR", "韩国", "South Korea", "A", "🇰🇷", 1840],
  ["cze", "CZE", "捷克", "Czech Republic", "A", "🇨🇿", 1770],
  ["can", "CAN", "加拿大", "Canada", "B", "🇨🇦", 1810],
  ["bih", "BIH", "波黑", "Bosnia and Herzegovina", "B", "🇧🇦", 1800],
  ["qat", "QAT", "卡塔尔", "Qatar", "B", "🇶🇦", 1720],
  ["sui", "SUI", "瑞士", "Switzerland", "B", "🇨🇭", 1875],
  ["bra", "BRA", "巴西", "Brazil", "C", "🇧🇷", 2010],
  ["mar", "MAR", "摩洛哥", "Morocco", "C", "🇲🇦", 1880],
  ["hai", "HAI", "海地", "Haiti", "C", "🇭🇹", 1640],
  ["sco", "SCO", "苏格兰", "Scotland", "C", "🏴", 1775],
  ["usa", "USA", "美国", "United States", "D", "🇺🇸", 1910],
  ["par", "PAR", "巴拉圭", "Paraguay", "D", "🇵🇾", 1740],
  ["aus", "AUS", "澳大利亚", "Australia", "D", "🇦🇺", 1830],
  ["tur", "TUR", "土耳其", "Turkey", "D", "🇹🇷", 1780],
  ["ger", "GER", "德国", "Germany", "E", "🇩🇪", 2025],
  ["cuw", "CUW", "库拉索", "Curacao", "E", "🇨🇼", 1580],
  ["ecu", "ECU", "厄瓜多尔", "Ecuador", "E", "🇪🇨", 1765],
  ["civ", "CIV", "科特迪瓦", "Ivory Coast", "E", "🇨🇮", 1805],
  ["swe", "SWE", "瑞典", "Sweden", "F", "🇸🇪", 1900],
  ["tun", "TUN", "突尼斯", "Tunisia", "F", "🇹🇳", 1710],
  ["ned", "NED", "荷兰", "Netherlands", "F", "🇳🇱", 1990],
  ["jpn", "JPN", "日本", "Japan", "F", "🇯🇵", 1885],
  ["irn", "IRN", "伊朗", "Iran", "G", "🇮🇷", 1830],
  ["nzl", "NZL", "新西兰", "New Zealand", "G", "🇳🇿", 1630],
  ["bel", "BEL", "比利时", "Belgium", "G", "🇧🇪", 1940],
  ["egy", "EGY", "埃及", "Egypt", "G", "🇪🇬", 1790],
  ["ksa", "KSA", "沙特阿拉伯", "Saudi Arabia", "H", "🇸🇦", 1700],
  ["uru", "URU", "乌拉圭", "Uruguay", "H", "🇺🇾", 1890],
  ["cpv", "CPV", "佛得角", "Cape Verde", "H", "🇨🇻", 1620],
  ["esp", "ESP", "西班牙", "Spain", "H", "🇪🇸", 2030],
  ["fra", "FRA", "法国", "France", "I", "🇫🇷", 2085],
  ["irq", "IRQ", "伊拉克", "Iraq", "I", "🇮🇶", 1680],
  ["nor", "NOR", "挪威", "Norway", "I", "🇳🇴", 1860],
  ["sen", "SEN", "塞内加尔", "Senegal", "I", "🇸🇳", 1825],
  ["alg", "ALG", "阿尔及利亚", "Algeria", "J", "🇩🇿", 1785],
  ["arg", "ARG", "阿根廷", "Argentina", "J", "🇦🇷", 2055],
  ["aut", "AUT", "奥地利", "Austria", "J", "🇦🇹", 1845],
  ["jor", "JOR", "约旦", "Jordan", "J", "🇯🇴", 1600],
  ["cod", "COD", "刚果民主共和国", "DR Congo", "K", "🇨🇩", 1660],
  ["col", "COL", "哥伦比亚", "Colombia", "K", "🇨🇴", 1905],
  ["por", "POR", "葡萄牙", "Portugal", "K", "🇵🇹", 2040],
  ["uzb", "UZB", "乌兹别克斯坦", "Uzbekistan", "K", "🇺🇿", 1615],
  ["cro", "CRO", "克罗地亚", "Croatia", "L", "🇭🇷", 1870],
  ["eng", "ENG", "英格兰", "England", "L", "🏴", 2020],
  ["gha", "GHA", "加纳", "Ghana", "L", "🇬🇭", 1685],
  ["pan", "PAN", "巴拿马", "Panama", "L", "🇵🇦", 1595]
] as const;

export const teams: Team[] = teamSeed.map(([id, code, nameZh, nameEn, group, flag, elo]) => ({
  id,
  code,
  nameZh,
  nameEn,
  group,
  flag,
  elo
}));

export const teamById = new Map(teams.map((team) => [team.id, team]));

export const matches: Match[] = [
  match("m001", "A", "小组赛", "2026-06-12", "03:00", "mex", "rsa", 2, 0, "finished", "Mexico City"),
  match("m002", "A", "小组赛", "2026-06-12", "10:00", "kor", "cze", 2, 1, "finished", "Guadalajara (Zapopan)"),
  match("m003", "B", "小组赛", "2026-06-13", "03:00", "can", "bih", 1, 1, "finished", "Toronto"),
  match("m004", "D", "小组赛", "2026-06-13", "09:00", "usa", "par", 4, 1, "finished", "Los Angeles (Inglewood)"),
  match("m005", "B", "小组赛", "2026-06-14", "03:00", "qat", "sui", 1, 1, "finished", "San Francisco Bay Area (Santa Clara)"),
  match("m006", "C", "小组赛", "2026-06-14", "06:00", "bra", "mar", 1, 1, "finished", "New York/New Jersey (East Rutherford)"),
  match("m007", "C", "小组赛", "2026-06-14", "09:00", "hai", "sco", 0, 1, "finished", "Boston (Foxborough)"),
  match("m008", "D", "小组赛", "2026-06-14", "12:00", "aus", "tur", 2, 0, "finished", "Vancouver"),
  match("m009", "E", "小组赛", "2026-06-15", "01:00", "ger", "cuw", 7, 1, "finished", "Houston"),
  match("m010", "F", "小组赛", "2026-06-15", "04:00", "ned", "jpn", 2, 2, "finished", "Dallas (Arlington)"),
  match("m011", "E", "小组赛", "2026-06-15", "07:00", "civ", "ecu", 1, 0, "finished", "Philadelphia"),
  match("m012", "F", "小组赛", "2026-06-15", "10:00", "swe", "tun", 5, 1, "finished", "Monterrey (Guadalupe)"),
  match("m013", "H", "小组赛", "2026-06-16", "03:00", "esp", "cpv", 0, 0, "finished", "Atlanta"),
  match("m014", "G", "小组赛", "2026-06-16", "06:00", "bel", "egy", 1, 1, "finished", "Seattle"),
  match("m015", "H", "小组赛", "2026-06-16", "09:00", "ksa", "uru", 1, 1, "finished", "Miami (Miami Gardens)"),
  match("m016", "G", "小组赛", "2026-06-16", "12:00", "irn", "nzl", 2, 2, "finished", "Los Angeles (Inglewood)"),
  match("m017", "I", "小组赛", "2026-06-17", "03:00", "fra", "sen", 3, 1, "finished", "New York/New Jersey (East Rutherford)"),
  match("m018", "I", "小组赛", "2026-06-17", "06:00", "irq", "nor", 1, 4, "finished", "Boston (Foxborough)"),
  match("m019", "J", "小组赛", "2026-06-17", "09:00", "arg", "alg", null, null, "scheduled", "Kansas City"),
  match("m020", "J", "小组赛", "2026-06-17", "12:00", "aut", "jor", null, null, "scheduled", "San Francisco Bay Area (Santa Clara)"),
  match("m021", "K", "小组赛", "2026-06-18", "03:00", "cod", "col", null, null, "scheduled", "Vancouver"),
  match("m022", "K", "小组赛", "2026-06-18", "06:00", "por", "uzb", null, null, "scheduled", "Houston"),
  match("m023", "L", "小组赛", "2026-06-18", "09:00", "cro", "eng", null, null, "scheduled", "Philadelphia"),
  match("m024", "L", "小组赛", "2026-06-18", "12:00", "gha", "pan", null, null, "scheduled", "Atlanta")
];

export const bracketNodes: BracketNode[] = [
  node("r32-1", "32", "2A", "2B"),
  node("r32-2", "32", "1C", "2F"),
  node("r32-3", "32", "1E", "3A/B/C/D/F"),
  node("r32-4", "32", "1F", "2C"),
  node("r32-5", "32", "2E", "2I"),
  node("r32-6", "32", "1I", "3C/D/F/G/H"),
  node("r32-7", "32", "1A", "3C/E/F/H/I"),
  node("r32-8", "32", "1L", "3E/H/I/J/K"),
  node("r32-9", "32", "1G", "3A/E/H/I/J"),
  node("r32-10", "32", "1D", "3B/E/F/I/J"),
  node("r32-11", "32", "1H", "2J"),
  node("r32-12", "32", "2K", "2L"),
  node("r32-13", "32", "1B", "3E/F/G/I/J"),
  node("r32-14", "32", "2D", "2G"),
  node("r32-15", "32", "1J", "2H"),
  node("r32-16", "32", "1K", "3D/E/I/J/L"),
  node("r16-1", "16", "W73", "W75"),
  node("r16-2", "16", "W74", "W77"),
  node("r16-3", "16", "W76", "W78"),
  node("r16-4", "16", "W79", "W80"),
  node("r16-5", "16", "W83", "W84"),
  node("r16-6", "16", "W81", "W82"),
  node("r16-7", "16", "W86", "W88"),
  node("r16-8", "16", "W85", "W87"),
  node("qf-1", "quarter", "W89", "W90"),
  node("qf-2", "quarter", "W93", "W94"),
  node("qf-3", "quarter", "W91", "W92"),
  node("qf-4", "quarter", "W99", "W100"),
  node("sf-1", "semi", "W97", "W98"),
  node("sf-2", "semi", "W95", "W96"),
  node("third", "third", "L101", "L102"),
  {
    id: "final",
    round: "final",
    slotA: "W101",
    slotB: "W102",
    venue: "New York/New Jersey (East Rutherford)"
  }
];

function match(
  id: string,
  group: string,
  stage: string,
  date: string,
  time: string,
  homeTeamId: string,
  awayTeamId: string,
  home: number | null,
  away: number | null,
  status: Match["status"],
  venue: string
): Match {
  return {
    id,
    group,
    stage,
    date,
    time,
    utcOffset: "UTC+8",
    homeTeamId,
    awayTeamId,
    score: { home, away },
    status,
    venue,
    city: venue,
    updatedAt: "15:43:04"
  };
}

function node(id: string, round: BracketNode["round"], slotA: string, slotB: string): BracketNode {
  return { id, round, slotA, slotB };
}

