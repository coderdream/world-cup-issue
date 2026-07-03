import sqlite3, os, json
path=os.environ['DB_PATH']
conn=sqlite3.connect(path)
conn.row_factory=sqlite3.Row
rows=conn.execute("select created_at, level, module, action, message, detail, trace_id from operate_log where trace_id like 'e2e%' order by id desc limit 80").fetchall()
for r in reversed(rows):
    print(json.dumps(dict(r), ensure_ascii=False))
