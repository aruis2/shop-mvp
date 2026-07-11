#!/bin/bash
# backup-db.sh — Backup automat PostgreSQL (NIST CSF: Recover)
# Rulează zilnic: 0 3 * * * /path/to/backup-db.sh

DATE=$(date +%Y-%m-%d)
BACKUP_DIR="${BACKUP_DIR:-/home/iuri/Desktop/2/backups}"
DB_URL="${DATABASE_URL:-postgresql://postgres:123123@localhost:5432/test}"
RETENTION_DAYS=30

mkdir -p "$BACKUP_DIR"

echo "[$(date)] Backup DB început..."

# Backup folosind pg_dump
pg_dump "$DB_URL" > "$BACKUP_DIR/shop-mvp-$DATE.sql" 2>/dev/null

if [ $? -eq 0 ]; then
    gzip -f "$BACKUP_DIR/shop-mvp-$DATE.sql"
    echo "[$(date)] ✅ Backup salvat: shop-mvp-$DATE.sql.gz"
else
    echo "[$(date)] ❌ Backup eșuat!"
    exit 1
fi

# Șterge backup-urile mai vechi de RETENTION_DAYS zile
find "$BACKUP_DIR" -name "shop-mvp-*.sql.gz" -mtime +$RETENTION_DAYS -delete
echo "[$(date)] 🧹 Backup-uri mai vechi de $RETENTION_DAYS zile șterse"

# Verifică dimensiunea
du -sh "$BACKUP_DIR/shop-mvp-$DATE.sql.gz"

echo "[$(date)] ✅ Backup complet"