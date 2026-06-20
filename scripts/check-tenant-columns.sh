#!/usr/bin/env bash
# Tenant-column presence check (docs/architecture/61-testing-strategy.md
# §"Continuous Integration" — "architecture-compliance check (... tenant-
# column presence on tenant-scoped tables)").
#
# Scans every CREATE TABLE statement in migrations/ and fails if the
# table is missing organization_id, unless it is explicitly listed
# as platform-global (doc 12 §"The organization_id Convention" —
# "Platform-global tables (e.g. modules, module_migrations, the
# permission registry) do not [carry organization_id]").
#
# ## What this does NOT do (yet)
#
# It does not read Module::manifest().tenant_scoped from compiled Rust
# code — no module has a populated manifest yet, so there is nothing
# real to introspect. This script is the SQL-level check the
# architecture doc calls for, sized to what actually exists today.
# A manifest-driven version (cross-checking tenant_scoped against the
# schema via SQLx introspection) is a reasonable later upgrade once
# modules have real manifests, not a requirement now.
#
# Usage: scripts/check-tenant-columns.sh
# Exit code: 0 if every non-exempt table has organization_id, 1 otherwise.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MIGRATIONS_DIR="${REPO_ROOT}/migrations"

# Platform-global tables exempt from the organization_id requirement
# (doc 12). Keep this list explicit and small — anything added here
# should be because doc 12 or doc 14 says so, not for convenience.
EXEMPT_TABLES=(
    "organizations"   # the tenant boundary itself; has no parent tenant
    "modules"
    "module_state"
    "module_migrations"
)

is_exempt() {
    local table="$1"
    for exempt in "${EXEMPT_TABLES[@]}"; do
        if [[ "$table" == "$exempt" ]]; then
            return 0
        fi
    done
    return 1
}

failures=0
checked=0

if [[ ! -d "$MIGRATIONS_DIR" ]]; then
    echo "No migrations directory found at ${MIGRATIONS_DIR} — nothing to check."
    exit 0
fi

while IFS= read -r -d '' file; do
    # Match "CREATE TABLE table_name (" allowing for "IF NOT EXISTS"
    # and case variation; grep -Pzo handles the multi-line table body.
    tables=$(grep -oiP 'CREATE TABLE\s+(IF NOT EXISTS\s+)?\K[a-zA-Z0-9_]+' "$file" || true)

    for table in $tables; do
        checked=$((checked + 1))

        if is_exempt "$table"; then
            continue
        fi

        # Extract the table's column block: from its CREATE TABLE line
        # to the matching closing ");" — approximated here by taking
        # everything up to the first line that is just ");" after the
        # CREATE TABLE statement starts. Good enough for this
        # project's migration style (one statement per logical block,
        # no nested parens spanning multiple tables in one file).
        body=$(awk -v t="$table" '
            BEGIN{found=0}
            tolower($0) ~ "create table.*[[:space:]]"tolower(t)"[[:space:]]*\\(" {found=1}
            found{print}
            found && /^\);/{exit}
        ' "$file")

        if ! echo "$body" | grep -qiP '\borganization_id\b'; then
            echo "FAIL: table '${table}' in $(basename "$file") has no organization_id column"
            failures=$((failures + 1))
        fi
    done
done < <(find "$MIGRATIONS_DIR" -name '*.sql' -print0)

echo "Checked ${checked} table(s); ${failures} failure(s)."

if [[ "$failures" -gt 0 ]]; then
    exit 1
fi