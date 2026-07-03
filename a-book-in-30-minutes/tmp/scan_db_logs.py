import sqlite3, os, re
path=os.environ['DB_PATH']
pat=re.compile(r'闁|鐠|閻|婵|濞|缂|鈧|锟|�|\?\?\?')
conn=sqlite3.connect(path)
rows=conn.execute("select id, message, detail, trace_id from operate_log where trace_id like 'e2e%' order by id desc limit 120").fetchall()
count=0
for row in rows:
    text=' '.join(str(v or '') for v in row)
    if pat.search(text):
        count+=1
        print(row)
print('garbled_count=', count)
