import sqlite3, os, json
path=os.environ['DB_PATH']
epub=r'D:\books\0625新书四本\2025-01《山茶的情书》\山茶的情书.epub'
conn=sqlite3.connect(path)
conn.row_factory=sqlite3.Row
rows=conn.execute("select path,status,progress,material_output_dir,message,audio_status,audio_progress,audio_file,audio_duration_ms,audio_chunks,audio_message,video_status,video_progress,video_file,video_duration_ms,video_file_size,video_message from material_tasks where path=?", (epub,)).fetchall()
for r in rows:
    print(json.dumps(dict(r), ensure_ascii=False))
