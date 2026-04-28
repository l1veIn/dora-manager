import json
import os
import sqlite3
import time


DB_PATH = os.path.expanduser("~/.dm/services/message/message.db")


class MessageDB:
    def __init__(self, path=DB_PATH):
        self.path = path
        os.makedirs(os.path.dirname(path), exist_ok=True)
        self.conn = sqlite3.connect(path)
        self.conn.execute("PRAGMA journal_mode=WAL")
        self.conn.execute("PRAGMA busy_timeout=5000")
        self.conn.execute(
            """
            CREATE TABLE IF NOT EXISTS messages (
                seq         INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id      TEXT NOT NULL,
                node_id     TEXT NOT NULL,
                tag         TEXT NOT NULL,
                payload     TEXT NOT NULL,
                timestamp   INTEGER NOT NULL
            )
            """
        )
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_messages_run ON messages(run_id, seq)"
        )
        self.conn.execute(
            """
            CREATE INDEX IF NOT EXISTS idx_messages_run_tag
            ON messages(run_id, node_id, tag, seq)
            """
        )
        self.conn.execute(
            """
            CREATE TABLE IF NOT EXISTS message_snapshots (
                run_id      TEXT NOT NULL,
                node_id     TEXT NOT NULL,
                tag         TEXT NOT NULL,
                payload     TEXT NOT NULL,
                seq         INTEGER NOT NULL,
                updated_at  INTEGER NOT NULL,
                PRIMARY KEY (run_id, node_id, tag)
            )
            """
        )

    def push(self, run_id, node_id, tag, payload, timestamp=None):
        ts = int(timestamp if timestamp is not None else time.time())
        payload_json = json.dumps(payload, ensure_ascii=False, separators=(",", ":"))
        with self.conn:
            cur = self.conn.execute(
                """
                INSERT INTO messages (run_id, node_id, tag, payload, timestamp)
                VALUES (?, ?, ?, ?, ?)
                """,
                (run_id, node_id, tag, payload_json, ts),
            )
            seq = cur.lastrowid
            self.conn.execute(
                """
                INSERT INTO message_snapshots
                    (run_id, node_id, tag, payload, seq, updated_at)
                VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT(run_id, node_id, tag) DO UPDATE SET
                    payload = excluded.payload,
                    seq = excluded.seq,
                    updated_at = excluded.updated_at
                """,
                (run_id, node_id, tag, payload_json, seq, ts),
            )
        return seq

    def list(
        self,
        run_id,
        after_seq=None,
        before_seq=None,
        from_filter=None,
        tag=None,
        limit=200,
        desc=False,
    ):
        conditions = ["run_id = ?"]
        params = [run_id]

        if after_seq is not None:
            conditions.append("seq > ?")
            params.append(int(after_seq))
        if before_seq is not None:
            conditions.append("seq < ?")
            params.append(int(before_seq))
        if from_filter is not None and "*" not in from_filter:
            if from_filter:
                placeholders = ", ".join("?" for _ in from_filter)
                conditions.append(f"node_id IN ({placeholders})")
                params.extend(from_filter)
            else:
                conditions.append("0")
        if tag is not None and "*" not in tag:
            if tag:
                placeholders = ", ".join("?" for _ in tag)
                conditions.append(f"tag IN ({placeholders})")
                params.extend(tag)
            else:
                conditions.append("0")

        order = "DESC" if desc else "ASC"
        max_rows = 200 if limit is None else int(limit)
        sql = f"""
            SELECT seq, node_id, tag, payload, timestamp
            FROM messages
            WHERE {' AND '.join(conditions)}
            ORDER BY seq {order}
            LIMIT ?
        """
        params.append(max_rows)

        rows = self.conn.execute(sql, params).fetchall()
        messages = [
            {
                "seq": row[0],
                "from": row[1],
                "tag": row[2],
                "payload": json.loads(row[3]),
                "timestamp": row[4],
            }
            for row in rows
        ]
        if desc:
            messages.reverse()

        next_seq = (
            messages[-1]["seq"]
            if messages
            else (int(after_seq) if after_seq is not None else 0)
        )
        return {"messages": messages, "next_seq": next_seq}

    def snapshots(self, run_id):
        rows = self.conn.execute(
            """
            SELECT node_id, tag, payload, seq, updated_at
            FROM message_snapshots
            WHERE run_id = ?
            ORDER BY node_id ASC, tag ASC
            """,
            (run_id,),
        ).fetchall()
        return [
            {
                "node_id": row[0],
                "tag": row[1],
                "payload": json.loads(row[2]),
                "seq": row[3],
                "updated_at": row[4],
            }
            for row in rows
        ]
