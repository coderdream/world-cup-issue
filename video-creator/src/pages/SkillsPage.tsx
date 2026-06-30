import { useEffect, useState } from "react";
import { frameworkApi } from "@/services/frameworkApi";
import type { SkillConfigEntry } from "@/types";
import { useDashboard } from "@/pages/useDashboard";

export function SkillsPage() {
  const { dashboard, setDashboard } = useDashboard();
  const [skills, setSkills] = useState<SkillConfigEntry[]>([]);
  const [message, setMessage] = useState("");

  useEffect(() => {
    if (dashboard) setSkills([...dashboard.skills].sort((a, b) => a.sortOrder - b.sortOrder));
  }, [dashboard]);

  function move(index: number, delta: number) {
    const next = [...skills];
    const target = index + delta;
    if (target < 0 || target >= next.length) return;
    [next[index], next[target]] = [next[target], next[index]];
    setSkills(next.map((item, order) => ({ ...item, sortOrder: order * 10 })));
  }

  async function save() {
    const saved = await frameworkApi.saveSkillConfigs(skills);
    setSkills(saved);
    setDashboard((current) => current ? { ...current, skills: saved } : current);
    setMessage("技能配置已保存。");
  }

  return (
    <section className="studio-page">
      <div className="toolbar right">
        <button type="button" onClick={() => setSkills((items) => items.map((item) => ({ ...item, enabled: true })))}>
          全部启用
        </button>
        <button type="button" onClick={() => setSkills((items) => items.map((item) => ({ ...item, enabled: false })))}>
          全部停用
        </button>
        <button type="button" onClick={() => void save()}>保存并生效</button>
      </div>
      <div className="table-wrap">
        <table>
          <thead>
            <tr><th>Key</th><th>技能标题</th><th>命令</th><th>启用</th><th>排序</th><th>说明</th><th>操作</th></tr>
          </thead>
          <tbody>
            {skills.map((skill, index) => (
              <tr key={skill.key}>
                <td>{skill.key}</td>
                <td>{skill.title}</td>
                <td>{skill.command}</td>
                <td>
                  <input
                    checked={skill.enabled}
                    onChange={(event) => setSkills((items) => items.map((item) => item.key === skill.key ? { ...item, enabled: event.target.checked } : item))}
                    type="checkbox"
                  />
                </td>
                <td>{skill.sortOrder}</td>
                <td>{skill.description}</td>
                <td>
                  <button type="button" onClick={() => move(index, -1)}>上移</button>
                  <button type="button" onClick={() => move(index, 1)}>下移</button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {message && <p className="run-message">{message}</p>}
      <p className="muted">上移和下移控制执行中心小按钮顺序。勾选启用后保存即可生效。</p>
    </section>
  );
}
