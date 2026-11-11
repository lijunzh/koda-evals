"""Write fixture/logs/ for this task.

Run from the task directory: python3 hidden/make_logs.py

Six days of logs from three hosts: the job scheduler (local time, US
Eastern, so the offset changes on 2026-11-01), the export worker (UTC) and
Postgres (UTC). The nightly export runs at 02:00 local and exports the
previous day's orders.

On 2026-11-03 a schema migration (`ALTER TABLE orders ALTER COLUMN total
TYPE numeric(14,2)`, application `migrate-0042`) starts at 06:55 UTC and
rewrites the table under an ACCESS EXCLUSIVE lock until 07:41 UTC. The
export's query waits on that lock and is cancelled at its 30-minute
deadline. The retry waits on the same lock, runs once it is released, and
then fails to upload: the run's upload credential was issued at 02:00 local
with a 45-minute lifetime. The scheduler's alert names only that last
error, a 403 ExpiredToken. The cause is the migration's lock, which only
the Postgres log shows, matched to the export by time or by backend pid.

Unrelated noise around it: Metabase's failed logins every day (one at
07:52 UTC on 11-03), web requests cancelled by statement timeout while the
table is locked, a transient connection reset on 11-01, checkpoints,
autovacuum, slow queries, worker metrics.
"""

import pathlib
import random
from datetime import datetime, timedelta, timezone
from zoneinfo import ZoneInfo

random.seed(20261111)

UTC = timezone.utc
EASTERN = ZoneInfo("America/New_York")
START = datetime(2026, 10, 30, 0, 0, tzinfo=UTC)
END = datetime(2026, 11, 4, 14, 0, tzinfo=UTC)
NIGHTLIES = [datetime(2026, 10, d, 2, 0, tzinfo=EASTERN) for d in (30, 31)] + [
    datetime(2026, 11, d, 2, 0, tzinfo=EASTERN) for d in (1, 2, 3, 4)
]
BAD = datetime(2026, 11, 3, 2, 0, tzinfo=EASTERN)
# The migration holds orders from 06:55:02 to 07:41:14.9 UTC on 11-03.
LOCKED = (datetime(2026, 11, 3, 6, 55, 2, 114000, tzinfo=UTC), datetime(2026, 11, 3, 7, 41, 14, 931000, tzinfo=UTC))

EXPORT_SQL = (
    "SELECT o.id, o.customer_id, o.created_at, o.total, o.currency, "
    "i.sku, i.qty, i.unit_price FROM orders o JOIN order_items i ON i.order_id = o.id "
    "WHERE o.created_at >= '{day}' AND o.created_at < '{next}' ORDER BY o.id"
)
ALTER_SQL = "ALTER TABLE orders ALTER COLUMN total TYPE numeric(14,2);"
WEB_SQL = [
    "SELECT * FROM orders WHERE customer_id = $1 ORDER BY created_at DESC LIMIT 20",
    "SELECT count(*) FROM orders WHERE status = 'pending'",
    "UPDATE orders SET status = $1 WHERE id = $2",
]

sched: list[tuple[datetime, str]] = []
worker: list[tuple[datetime, str]] = []
pg: list[tuple[datetime, str]] = []

pid = 28000


def next_pid() -> int:
    global pid
    pid += random.randint(3, 40)
    return pid


def ms(t: datetime) -> str:
    return f"{t.microsecond // 1000:03d}"


def s_line(t: datetime, level: str, msg: str) -> None:
    lt = t.astimezone(EASTERN)
    off = lt.strftime("%z")
    off = off[:3] + ":" + off[3:]
    sched.append((t, f"{lt:%Y-%m-%dT%H:%M:%S}.{ms(lt)}{off} {level:<5} {msg}"))


def w_line(t: datetime, level: str, msg: str) -> None:
    worker.append((t, f"{t:%Y-%m-%dT%H:%M:%S}.{ms(t)}Z {level:<5} [exp-worker-2] {msg}"))


def p_line(t: datetime, p: int, who: str, level: str, msg: str) -> None:
    pg.append((t, f"{t:%Y-%m-%d %H:%M:%S}.{ms(t)} UTC [{p}] {who} {level}:  {msg}"))


def jitter(t: datetime, lo: float = 0.01, hi: float = 0.9) -> datetime:
    return t + timedelta(seconds=random.uniform(lo, hi))


# Scheduler: inventory sync every 15 minutes, hourly rollup at :05.
t = START
while t < END:
    if t.minute % 15 == 0:
        a = jitter(t)
        s_line(a, "INFO", "job=sync-inventory started on host=sched-1")
        dur = random.uniform(4, 14)
        if random.random() < 0.02:
            s_line(a + timedelta(seconds=dur), "WARN", f"job=sync-inventory slow: {dur:.1f}s (budget 10s)")
        s_line(a + timedelta(seconds=dur), "INFO", f"job=sync-inventory finished ok in {dur:.1f}s")
    if t.minute == 5:
        a = jitter(t)
        s_line(a, "INFO", "job=rollup-hourly started on host=exp-worker-2")
        w_line(a + timedelta(seconds=0.2), "INFO", "job=rollup-hourly start")
        dur = random.uniform(20, 55)
        rows = random.randint(9000, 31000)
        w_line(a + timedelta(seconds=dur - 0.3), "INFO", f"job=rollup-hourly wrote {rows} rows to orders_hourly")
        s_line(a + timedelta(seconds=dur), "INFO", f"job=rollup-hourly finished ok in {dur:.0f}s")
        p = next_pid()
        p_line(a + timedelta(seconds=0.3), p, "rollup@shop app=rollup-hourly", "LOG",
               "connection authorized: user=rollup database=shop application_name=rollup-hourly")
        p_line(a + timedelta(seconds=dur - 0.2), p, "rollup@shop app=rollup-hourly", "LOG",
               f"disconnection: session time: 0:00:{dur:06.3f} user=rollup database=shop host=10.0.4.12 port={random.randint(40000, 60000)}")
    t += timedelta(minutes=5)

# Worker metrics every two minutes.
t = START
while t < END:
    w_line(jitter(t), "INFO", f"metrics rss_mb={random.randint(310, 480)} open_fds={random.randint(40, 90)} "
           f"tmp_used_gb={random.uniform(0.2, 1.1):.1f}")
    t += timedelta(minutes=2)

# Postgres housekeeping: checkpoints, autovacuum, slow web queries, Metabase.
t = START
while t < END:
    c = jitter(t, 0, 0.5)
    p_line(c, 61, "@", "LOG", "checkpoint starting: time")
    secs = random.uniform(25, 140)
    p_line(c + timedelta(seconds=secs), 61, "@", "LOG",
           f"checkpoint complete: wrote {random.randint(800, 9000)} buffers ({random.uniform(0.5, 6):.1f}%); "
           f"0 WAL file(s) added, 0 removed, {random.randint(0, 3)} recycled; write={secs - 0.4:.3f} s, sync=0.2{random.randint(10, 99)} s, total={secs:.3f} s")
    if random.random() < 0.08:
        table = random.choice(["order_items", "inventory", "sessions", "orders_hourly"])
        p_line(c + timedelta(seconds=random.uniform(30, 250)), next_pid(), "@", "LOG",
               f'automatic vacuum of table "shop.public.{table}": index scans: 1, pages: 0 removed, '
               f"{random.randint(2000, 90000)} remain; tuples: {random.randint(100, 9000)} removed")
    slow_at = c + timedelta(seconds=random.uniform(10, 280))
    if random.random() < 0.15 and not LOCKED[0] <= slow_at <= LOCKED[1]:
        wp = next_pid()
        d = random.uniform(1001, 4800)
        p_line(slow_at, wp, "web@shop app=web", "LOG",
               f"duration: {d:.3f} ms  statement: {random.choice(WEB_SQL)}")
    t += timedelta(minutes=5)

for day in range(30, 36):
    d = datetime(2026, 10, 30, tzinfo=UTC) + timedelta(days=day - 30)
    for _ in range(random.randint(1, 3)):
        at = d + timedelta(seconds=random.uniform(0, 86399))
        if START <= at < END:
            p_line(at, next_pid(), "metabase@shop app=[unknown]", "FATAL",
                   'password authentication failed for user "metabase"')
p_line(datetime(2026, 11, 3, 7, 52, 14, 377000, tzinfo=UTC), next_pid(), "metabase@shop app=[unknown]", "FATAL",
       'password authentication failed for user "metabase"')


def export_query(day: datetime) -> str:
    ed = (day - timedelta(days=1)).date()
    return EXPORT_SQL.format(day=ed.isoformat(), next=(ed + timedelta(days=1)).isoformat())


def stream_rows(t: datetime, run: str, until_rows: int, minutes: float) -> datetime:
    """Progress lines while rows stream; returns the time the query ends."""
    end = t + timedelta(minutes=minutes)
    n = 0
    while t < end:
        t += timedelta(seconds=random.uniform(35, 45))
        n = min(until_rows, n + until_rows * 40 // int(minutes * 60))
        w_line(t, "INFO", f"job=nightly-export run={run} fetched {n} rows")
    return t


def upload(t: datetime, run: str, edate: str, parts: int) -> datetime:
    for i in range(parts):
        t += timedelta(seconds=random.uniform(4, 9))
        w_line(t, "INFO", f"job=nightly-export run={run} uploaded s3://shop-exports/daily/{edate}/part-{i:04d}.parquet")
    return t


for local in NIGHTLIES:
    t0 = local.astimezone(UTC)
    run = local.strftime("%Y%m%d")
    edate = (local - timedelta(days=1)).date().isoformat()
    expires = local + timedelta(minutes=45)
    s_line(jitter(t0, 0, 0.05), "INFO",
           f"job=nightly-export run={run} attempt=1/2 started on host=exp-worker-2; "
           f"upload credential cred=exp-{run} expires={expires.isoformat(timespec='seconds')}")
    p = next_pid()
    a = jitter(t0, 0.3, 0.5)
    w_line(a, "INFO", f"job=nightly-export run={run} attempt=1 start: export orders for {edate}")
    p_line(a + timedelta(seconds=0.1), p, "export@shop app=nightly-export", "LOG",
           "connection authorized: user=export database=shop application_name=nightly-export")
    w_line(a + timedelta(seconds=0.2), "INFO", f"job=nightly-export run={run} connected to postgres db-1 (backend pid {p})")
    w_line(a + timedelta(seconds=0.3), "INFO", f"job=nightly-export run={run} query started (deadline 1800s)")
    rows = random.randint(1_050_000, 1_350_000)

    if local != BAD:
        if local.day == 1:
            # A transient reset that recovers: noise before the real day.
            r = a + timedelta(minutes=4, seconds=12)
            w_line(r, "WARN", f"job=nightly-export run={run} read from postgres failed: connection reset by peer; reconnecting (1/3)")
            p_line(r - timedelta(seconds=0.1), p, "export@shop app=nightly-export", "LOG",
                   "could not receive data from client: Connection reset by peer")
            p = next_pid()
            p_line(r + timedelta(seconds=1.1), p, "export@shop app=nightly-export", "LOG",
                   "connection authorized: user=export database=shop application_name=nightly-export")
            w_line(r + timedelta(seconds=1.2), "INFO", f"job=nightly-export run={run} connected to postgres db-1 (backend pid {p})")
            w_line(r + timedelta(seconds=1.3), "INFO", f"job=nightly-export run={run} query restarted (deadline 1800s)")
            a = r + timedelta(seconds=1.3)
        mins = random.uniform(10, 13)
        q_end = stream_rows(a, run, rows, mins)
        w_line(q_end, "INFO", f"job=nightly-export run={run} query done: {rows} rows in {(q_end - a).total_seconds():.0f}s")
        p_line(q_end, p, "export@shop app=nightly-export", "LOG",
               f"duration: {(q_end - a).total_seconds() * 1000:.3f} ms  statement: {export_query(local)}")
        parts = rows // 100_000 + 1
        w_line(q_end + timedelta(seconds=2), "INFO", f"job=nightly-export run={run} wrote {parts} parquet parts to /var/tmp/export/{run}")
        u_end = upload(q_end + timedelta(seconds=40), run, edate, parts)
        w_line(u_end + timedelta(seconds=0.5), "INFO", f"job=nightly-export run={run} done exit=0")
        s_line(u_end + timedelta(seconds=0.9), "INFO",
               f"job=nightly-export run={run} finished ok in {(u_end - t0).total_seconds() / 60:.0f}m; {rows} rows, {parts} parts")
        continue

    # 2026-11-03: the migration's connection, then its lock on orders.
    mp = next_pid()
    m_conn = datetime(2026, 11, 3, 6, 54, 58, 902000, tzinfo=UTC)
    m_start, m_end = LOCKED
    who = "migrate@shop app=migrate-0042"
    p_line(m_conn, mp, who, "LOG", "connection authorized: user=migrate database=shop application_name=migrate-0042")
    p_line(m_start, mp, who, "LOG", f"statement: {ALTER_SQL}")
    # Web requests pile up behind the lock and hit their 30s statement timeout.
    wt = m_start + timedelta(seconds=31)
    while wt < m_end:
        wp = next_pid()
        p_line(wt, wp, "web@shop app=web", "ERROR", "canceling statement due to statement timeout")
        p_line(wt, wp, "web@shop app=web", "STATEMENT", random.choice(WEB_SQL))
        wt += timedelta(seconds=random.uniform(50, 160))

    def waits(pid_: int, since: datetime, run_: str) -> None:
        at = since + timedelta(seconds=1.0004)
        p_line(at, pid_, "export@shop app=nightly-export", "LOG",
               f"process {pid_} still waiting for AccessShareLock on relation 16412 of database 16384 after 1000.{random.randint(80, 140):03d} ms")
        p_line(at, pid_, "export@shop app=nightly-export", "DETAIL", f"Process holding the lock: {mp}. Wait queue: {pid_}.")
        p_line(at, pid_, "export@shop app=nightly-export", "STATEMENT", export_query(local))
        k = since
        while True:
            k += timedelta(seconds=60)
            if k >= m_end or (run_ == "1" and k >= since + timedelta(seconds=1800)):
                break
            w_line(k, "INFO", f"job=nightly-export run={run} query running, 0 rows fetched ({(k - since).total_seconds():.0f}s)")

    q1 = a + timedelta(seconds=0.3)
    waits(p, q1, "1")
    cancel = q1 + timedelta(seconds=1800)
    w_line(cancel, "ERROR", f"job=nightly-export run={run} query exceeded deadline 1800s; cancelling")
    p_line(cancel + timedelta(seconds=0.02), p, "export@shop app=nightly-export", "ERROR", "canceling statement due to user request")
    p_line(cancel + timedelta(seconds=0.02), p, "export@shop app=nightly-export", "STATEMENT", export_query(local))
    w_line(cancel + timedelta(seconds=0.1), "ERROR", f"job=nightly-export run={run} attempt=1 failed exit=3")
    s_line(cancel + timedelta(seconds=0.6), "WARN",
           f"job=nightly-export run={run} attempt=1/2 exited 3 after 30m01s (deadline exceeded); retrying in 60s")

    a2 = cancel + timedelta(seconds=60.6)
    s_line(a2 - timedelta(seconds=0.2), "INFO",
           f"job=nightly-export run={run} attempt=2/2 started on host=exp-worker-2; reusing cred=exp-{run}")
    w_line(a2, "INFO", f"job=nightly-export run={run} attempt=2 start: export orders for {edate}")
    p2 = next_pid()
    p_line(a2 + timedelta(seconds=0.1), p2, "export@shop app=nightly-export", "LOG",
           "connection authorized: user=export database=shop application_name=nightly-export")
    w_line(a2 + timedelta(seconds=0.2), "INFO", f"job=nightly-export run={run} connected to postgres db-1 (backend pid {p2})")
    w_line(a2 + timedelta(seconds=0.3), "INFO", f"job=nightly-export run={run} query started (deadline 1800s)")
    q2 = a2 + timedelta(seconds=0.3)
    waits(p2, q2, "2")

    p_line(m_end, mp, who, "LOG", f"duration: {(m_end - m_start).total_seconds() * 1000:.3f} ms  statement: {ALTER_SQL}")
    p_line(m_end + timedelta(seconds=0.05), p2, "export@shop app=nightly-export", "LOG",
           f"process {p2} acquired AccessShareLock on relation 16412 of database 16384 after {(m_end - q2).total_seconds() * 1000:.3f} ms")
    p_line(m_end + timedelta(seconds=0.05), p2, "export@shop app=nightly-export", "STATEMENT", export_query(local))
    p_line(m_end + timedelta(seconds=0.4), mp, who, "LOG",
           "disconnection: session time: 0:46:16.433 user=migrate database=shop host=10.0.7.31 port=51544")

    q_end = stream_rows(m_end + timedelta(seconds=0.1), run, rows, 8.2)
    w_line(q_end, "INFO", f"job=nightly-export run={run} query done: {rows} rows in {(q_end - q2).total_seconds():.0f}s")
    p_line(q_end, p2, "export@shop app=nightly-export", "LOG",
           f"duration: {(q_end - q2).total_seconds() * 1000:.3f} ms  statement: {export_query(local)}")
    parts = rows // 100_000 + 1
    w_line(q_end + timedelta(seconds=2), "INFO", f"job=nightly-export run={run} wrote {parts} parquet parts to /var/tmp/export/{run}")
    up = q_end + timedelta(seconds=41.3)
    w_line(up, "ERROR",
           f"job=nightly-export run={run} upload s3://shop-exports/daily/{edate}/part-0000.parquet failed: "
           f"403 ExpiredToken: The provided token has expired. (cred=exp-{run})")
    w_line(up + timedelta(seconds=0.1), "ERROR", f"job=nightly-export run={run} attempt=2 failed exit=2")
    w_line(up + timedelta(seconds=0.3), "WARN",
           f"job=nightly-export run={run} cleanup: leaving {parts} parts in /var/tmp/export/{run} for a manual retry")
    s_line(up + timedelta(seconds=0.8), "ERROR",
           f"job=nightly-export run={run} FAILED: attempt=2/2 exited 2: upload failed: 403 ExpiredToken (cred=exp-{run})")
    s_line(up + timedelta(seconds=0.9), "ERROR",
           f"alert sent to #data-oncall: nightly-export {run} FAILED (last error: upload failed: 403 ExpiredToken)")

out = pathlib.Path("fixture/logs")
out.mkdir(parents=True, exist_ok=True)
for name, lines in [("scheduler.log", sched), ("export-worker.log", worker), ("postgres.log", pg)]:
    lines.sort(key=lambda x: x[0])
    (out / name).write_text("".join(line + "\n" for _, line in lines))
    print(f"{name}: {len(lines)} lines")
