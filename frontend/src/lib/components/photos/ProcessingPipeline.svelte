<script lang="ts">
  import type { ProcessingReport } from '$lib/api/types';
  import ProcessingCurveChart from './ProcessingCurveChart.svelte';

  let { report }: { report: ProcessingReport } = $props();

  function fmtDuration(s: number | null): string {
    if (s == null) return '';
    if (s < 60) return `${s < 10 ? s.toFixed(1) : Math.round(s)}s`;
    const m = Math.floor(s / 60);
    const rem = Math.round(s % 60);
    return rem ? `${m}m ${rem}s` : `${m}m`;
  }

  const headline = $derived(
    [
      report.creatorApp,
      `${report.pipeline.length} step${report.pipeline.length === 1 ? '' : 's'}`,
      report.totalDurationS != null ? `${fmtDuration(report.totalDurationS)} compute` : null
    ]
      .filter(Boolean)
      .join('  ·  ')
  );
</script>

<section class="processing">
  <div class="processing-header">
    <span class="t-label">PROCESSING</span>
  </div>
  {#if headline}<p class="headline">{headline}</p>{/if}

  <ol class="timeline">
    {#each report.pipeline as step (step.position)}
      <li class:disabled={!step.enabled}>
        <details>
          <summary>
            <span class="marker" aria-hidden="true"></span>
            <span class="head">
              <span class="label">{step.label}</span>
              <span class="cat">{step.category}</span>
            </span>
            <span class="meta">
              {#if !step.enabled}<span class="badge">disabled</span>{/if}
              {#if step.durationS != null}<span class="dur">{fmtDuration(step.durationS)}</span>{/if}
            </span>
          </summary>

          <div class="body">
            {#if step.summary}<p class="desc">{step.summary}</p>{/if}

            {#if step.params.length > 0}
              <dl class="params">
                {#each step.params as p (p.key)}
                  <dt>{p.key}</dt>
                  <dd class:truncated={p.truncated}>{p.value}</dd>
                {/each}
              </dl>
            {/if}

            {#each step.tables as t (t.id)}
              {#if t.kind === 'curve'}
                <figure class="chart">
                  <ProcessingCurveChart table={t} />
                  <figcaption>{t.id} curve · {t.rows.length} points</figcaption>
                </figure>
              {:else if t.rows.length > 0}
                <div class="tablewrap">
                  <table class="datatable">
                    {#if t.columns.length}
                      <thead><tr>{#each t.columns as c (c)}<th>{c}</th>{/each}</tr></thead>
                    {/if}
                    <tbody>
                      {#each t.rows as row, ri (ri)}
                        <tr>{#each row as cell, ci (ci)}<td>{cell}</td>{/each}</tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {/if}
            {/each}
          </div>
        </details>
      </li>
    {/each}
  </ol>
</section>

<style>
  .processing {
    margin-top: 2rem;
  }
  .processing-header {
    margin-bottom: 0.35rem;
  }
  .t-label {
    font-family: var(--font-ui);
    font-size: 0.72rem;
    letter-spacing: 0.08em;
    color: var(--fg-muted);
    font-weight: 600;
  }
  .headline {
    margin: 0 0 1rem;
    font-family: var(--font-mono);
    font-size: 0.8rem;
    color: var(--fg-secondary);
  }

  .timeline {
    list-style: none;
    margin: 0;
    padding: 0;
    border-left: 1px solid var(--border-default);
  }
  .timeline li {
    position: relative;
  }
  .timeline li.disabled {
    opacity: 0.5;
  }

  summary {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    padding: 0.5rem 0.5rem 0.5rem 1.1rem;
    cursor: pointer;
    list-style: none;
    border-radius: var(--r-md);
  }
  summary::-webkit-details-marker {
    display: none;
  }
  summary:hover {
    background: var(--bg-elevated);
  }
  .marker {
    position: absolute;
    left: -4.5px;
    top: 0.85rem;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 3px var(--bg-base);
  }
  .head {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    flex: 1;
    min-width: 0;
  }
  .label {
    font-weight: 600;
    color: var(--fg-primary);
  }
  .cat {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--fg-faint);
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    white-space: nowrap;
  }
  .dur {
    font-family: var(--font-mono);
    font-size: 0.78rem;
    color: var(--fg-muted);
  }
  .badge {
    font-size: 0.66rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    background: var(--bg-raised);
    color: var(--fg-muted);
    padding: 0.05rem 0.4rem;
    border-radius: var(--r-pill);
  }

  .body {
    padding: 0 0.5rem 0.75rem 1.1rem;
  }
  .desc {
    margin: 0 0 0.6rem;
    color: var(--fg-secondary);
    font-size: 0.85rem;
  }
  .params {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 0.18rem 1rem;
    margin: 0;
    font-size: 0.82rem;
  }
  .params dt {
    color: var(--fg-muted);
    font-family: var(--font-mono);
  }
  .params dd {
    margin: 0;
    color: var(--fg-secondary);
    font-family: var(--font-mono);
    word-break: break-word;
  }
  .params dd.truncated {
    font-style: italic;
    color: var(--fg-faint);
  }

  .chart {
    margin: 0.75rem 0 0;
  }
  .chart figcaption {
    font-size: 0.72rem;
    color: var(--fg-faint);
    font-family: var(--font-mono);
    margin-top: 0.2rem;
  }
  .tablewrap {
    margin-top: 0.6rem;
    overflow-x: auto;
  }
  .datatable {
    border-collapse: collapse;
    font-size: 0.78rem;
    font-family: var(--font-mono);
  }
  .datatable th,
  .datatable td {
    padding: 0.1rem 0.6rem 0.1rem 0;
    text-align: right;
    color: var(--fg-secondary);
  }
  .datatable th {
    color: var(--fg-faint);
    font-weight: 500;
    border-bottom: 1px solid var(--border-subtle);
  }
</style>
