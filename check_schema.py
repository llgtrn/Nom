import sqlite3
c = sqlite3.connect('nom-compiler/nomdict.db')
cursor = c.execute("SELECT name, sql FROM sqlite_master WHERE type='table'")
for row in cursor.fetchall():
    print(f"TABLE: {row[0]}")
    print(row[1] if row[1] else "(no sql)")
    print("---")
