//! HTML template constants for the `HtmlFormatter`.
//!
//! Separates CSS styles and JavaScript code from the core formatting logic.

/// HTML document header including all CSS styles.
pub const HTML_HEADER: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>SLOC Guard Report</title>
    <style>
        :root {
            --color-passed: #22c55e;
            --color-warning: #eab308;
            --color-failed: #ef4444;
            --color-grandfathered: #3b82f6;
            --color-bg: #f8fafc;
            --color-card: #ffffff;
            --color-border: #e2e8f0;
            --color-text: #1e293b;
            --color-text-muted: #64748b;
            --color-chart-primary: #6366f1;
            --color-delta-good: #22c55e;
            --color-delta-bad: #ef4444;
            --color-delta-neutral: #64748b;
        }
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            background: var(--color-bg);
            color: var(--color-text);
            line-height: 1.6;
            padding: 2rem;
        }
        .container { max-width: 1200px; margin: 0 auto; }
        h1 { font-size: 1.875rem; font-weight: 700; margin-bottom: 1.5rem; color: var(--color-text); }
        h2 { font-size: 1.25rem; font-weight: 600; margin: 1.5rem 0 1rem; color: var(--color-text); }
        .summary-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 1rem; margin-bottom: 2rem; }
        .summary-card { background: var(--color-card); border-radius: 0.5rem; padding: 1.25rem; border: 1px solid var(--color-border); text-align: center; }
        .summary-card .value { font-size: 2rem; font-weight: 700; display: block; }
        .summary-card .label { font-size: 0.875rem; color: var(--color-text-muted); margin-top: 0.25rem; }
        .summary-card.passed .value { color: var(--color-passed); }
        .summary-card.warning .value { color: var(--color-warning); }
        .summary-card.failed .value { color: var(--color-failed); }
        .summary-card.grandfathered .value { color: var(--color-grandfathered); }
        .filter-controls { display: flex; gap: 0.5rem; margin-bottom: 1rem; flex-wrap: wrap; }
        .filter-btn { padding: 0.5rem 1rem; border: 1px solid var(--color-border); background: var(--color-card); border-radius: 0.375rem; cursor: pointer; font-size: 0.875rem; transition: all 0.15s; }
        .filter-btn:hover { background: var(--color-bg); }
        .filter-btn.active { background: var(--color-text); color: var(--color-card); border-color: var(--color-text); }
        .table-container { overflow-x: auto; }
        table { width: 100%; border-collapse: collapse; background: var(--color-card); border-radius: 0.5rem; overflow: hidden; border: 1px solid var(--color-border); }
        th, td { padding: 0.75rem 1rem; text-align: left; border-bottom: 1px solid var(--color-border); }
        th { background: var(--color-bg); font-weight: 600; font-size: 0.875rem; color: var(--color-text-muted); text-transform: uppercase; letter-spacing: 0.05em; }
        th.sortable { cursor: pointer; user-select: none; }
        th.sortable:hover { background: #e2e8f0; }
        th.sortable::after { content: ''; display: inline-block; width: 0; height: 0; margin-left: 0.5rem; vertical-align: middle; opacity: 0.3; }
        th.sortable.asc::after { border-left: 4px solid transparent; border-right: 4px solid transparent; border-bottom: 6px solid currentColor; opacity: 1; }
        th.sortable.desc::after { border-left: 4px solid transparent; border-right: 4px solid transparent; border-top: 6px solid currentColor; opacity: 1; }
        td { font-size: 0.875rem; }
        td.number { text-align: right; font-variant-numeric: tabular-nums; }
        tr:last-child td { border-bottom: none; }
        tbody tr:hover { background: var(--color-bg); }
        tr.hidden { display: none; }
        .status { display: inline-flex; align-items: center; gap: 0.375rem; padding: 0.25rem 0.625rem; border-radius: 9999px; font-size: 0.75rem; font-weight: 600; }
        .status.passed { background: #dcfce7; color: #166534; }
        .status.warning { background: #fef9c3; color: #854d0e; }
        .status.failed { background: #fee2e2; color: #991b1b; }
        .status.grandfathered { background: #dbeafe; color: #1e40af; }
        .file-path { font-family: 'SF Mono', SFMono-Regular, Consolas, 'Liberation Mono', Menlo, monospace; font-size: 0.8125rem; word-break: break-all; }
        .reason { font-size: 0.75rem; color: var(--color-text-muted); font-style: italic; }
        .suggestions { margin-top: 0.5rem; padding: 0.75rem; background: var(--color-bg); border-radius: 0.375rem; font-size: 0.75rem; }
        .suggestions h4 { font-size: 0.75rem; font-weight: 600; margin-bottom: 0.375rem; }
        .suggestions ul { list-style: none; margin: 0; padding: 0; }
        .suggestions li { padding: 0.25rem 0; font-family: 'SF Mono', SFMono-Regular, Consolas, monospace; }
        .footer { margin-top: 2rem; padding-top: 1rem; border-top: 1px solid var(--color-border); font-size: 0.75rem; color: var(--color-text-muted); text-align: center; }
        .no-results { padding: 2rem; text-align: center; color: var(--color-text-muted); }
        .charts-section { margin: 2rem 0; }
        .chart-container { background: var(--color-card); border-radius: 0.5rem; padding: 1.25rem; border: 1px solid var(--color-border); margin-bottom: 1rem; }
        .chart-container h3 { font-size: 1rem; font-weight: 600; margin-bottom: 1rem; color: var(--color-text); }
        .chart-container svg { width: 100%; height: auto; max-width: 500px; }
        /* SVG chart hover effects */
        .chart-container svg rect { transition: opacity 0.15s ease; }
        .chart-container svg rect:hover { opacity: 0.85; }
        .chart-container svg circle { transition: r 0.15s ease, stroke-width 0.15s ease; }
        .chart-container svg circle:hover { stroke-width: 3; }
        /* Delta indicators */
        .delta-good { fill: var(--color-delta-good); }
        .delta-bad { fill: var(--color-delta-bad); }
        .delta-neutral { fill: var(--color-delta-neutral); }
        /* Print styles: ensure information isn't lost without color */
        @media print {
            body { background: white; color: black; padding: 1rem; }
            .summary-card, .chart-container, table { border: 1px solid #333; }
            .summary-card .value { color: inherit !important; }
            .summary-card.passed .label::before { content: '✓ '; }
            .summary-card.warning .label::before { content: '⚠ '; }
            .summary-card.failed .label::before { content: '✗ '; }
            .summary-card.grandfathered .label::before { content: '◉ '; }
            .filter-controls { display: none; }
            .status { background: transparent !important; border: 1px solid currentColor; }
            /* Print-friendly delta indicators with text labels */
            svg text.delta-label { display: inline !important; }
            /* Note: ::after pseudo-elements don't work with SVG <text> elements.
               Delta values are shown directly in the data-delta attribute for screen readers. */
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>SLOC Guard Report</h1>
"#;

/// HTML document footer including JavaScript for interactivity.
pub const HTML_FOOTER: &str = r#"        <div class="footer">
            Generated by <strong>sloc-guard</strong>
        </div>
    </div>
    <script>
        (function() {
            // Status filter functionality
            const filterBtns = document.querySelectorAll('.filter-btn');
            const fileTable = document.getElementById('file-table');
            if (filterBtns.length && fileTable) {
                filterBtns.forEach(btn => {
                    btn.addEventListener('click', () => {
                        filterBtns.forEach(b => b.classList.remove('active'));
                        btn.classList.add('active');
                        const filter = btn.dataset.filter;
                        const rows = fileTable.querySelectorAll('tbody tr');
                        rows.forEach(row => {
                            const status = row.dataset.status;
                            if (filter === 'all') {
                                row.classList.remove('hidden');
                            } else if (filter === 'issues') {
                                row.classList.toggle('hidden', status === 'passed');
                            } else {
                                row.classList.toggle('hidden', status !== filter);
                            }
                        });
                    });
                });
            }

            // Sortable columns functionality
            const sortableHeaders = document.querySelectorAll('th.sortable');
            sortableHeaders.forEach(header => {
                header.addEventListener('click', () => {
                    const table = header.closest('table');
                    const tbody = table.querySelector('tbody');
                    const rows = Array.from(tbody.querySelectorAll('tr'));
                    const colIndex = Array.from(header.parentNode.children).indexOf(header);
                    const isAsc = header.classList.contains('asc');

                    // Update sort indicators
                    table.querySelectorAll('th.sortable').forEach(th => {
                        th.classList.remove('asc', 'desc');
                    });
                    header.classList.add(isAsc ? 'desc' : 'asc');

                    // Sort rows
                    const sortType = header.dataset.sort;
                    rows.sort((a, b) => {
                        let aVal, bVal;
                        if (sortType === 'status') {
                            const order = {failed: 0, warning: 1, grandfathered: 2, passed: 3};
                            aVal = order[a.dataset.status] ?? 4;
                            bVal = order[b.dataset.status] ?? 4;
                        } else if (sortType === 'number') {
                            aVal = parseInt(a.children[colIndex].dataset.value, 10) || 0;
                            bVal = parseInt(b.children[colIndex].dataset.value, 10) || 0;
                        } else {
                            aVal = a.children[colIndex].textContent.trim().toLowerCase();
                            bVal = b.children[colIndex].textContent.trim().toLowerCase();
                        }
                        if (aVal < bVal) return isAsc ? 1 : -1;
                        if (aVal > bVal) return isAsc ? -1 : 1;
                        return 0;
                    });

                    rows.forEach(row => tbody.appendChild(row));
                });
            });
        })();
    </script>
</body>
</html>
"#;
