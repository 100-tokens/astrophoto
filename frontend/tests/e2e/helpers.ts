/**
 * Shared helpers for the EDGE_CASES Playwright suite.
 *
 * URLs are env-overridable. Defaults target the canonical `just dev` stack
 * (frontend :5173, backend :8080). When those ports are held by another local
 * app, run the astrophoto stack on alt ports and point the suite at them via
 * PLAYWRIGHT_BASE_URL / PLAYWRIGHT_BACKEND_URL (and PGPORT for the DB).
 */
import { execFileSync } from 'node:child_process';
import type { Page, APIRequestContext } from '@playwright/test';

export const FRONTEND = process.env.PLAYWRIGHT_BASE_URL ?? 'http://localhost:5173';
export const BACKEND = process.env.PLAYWRIGHT_BACKEND_URL ?? 'http://localhost:8080';
export const MAILHOG = process.env.PLAYWRIGHT_MAILHOG_URL ?? 'http://localhost:8025';

const PG = {
  host: process.env.PGHOST ?? 'localhost',
  port: process.env.PGPORT ?? '5434',
  user: process.env.PGUSER ?? 'astrophoto',
  db: process.env.PGDATABASE ?? 'astrophoto',
  password: process.env.PGPASSWORD ?? 'astrophoto'
};

/** Run a single SQL statement against the dev DB via psql. Returns stdout. */
export function sql(statement: string): string {
  return execFileSync(
    'psql',
    ['-h', PG.host, '-p', PG.port, '-U', PG.user, '-d', PG.db, '-tAc', statement],
    { env: { ...process.env, PGPASSWORD: PG.password }, encoding: 'utf8' }
  ).trim();
}

/** Mark a user's email verified (signin is blocked until verified_at is set). */
export function verifyEmail(email: string): void {
  sql(`update users set email_verified_at = now() where email = '${email}'`);
}

/** Grant a user the admin role. */
export function makeAdmin(email: string): void {
  sql(`update users set is_admin = true where email = '${email}'`);
}

export interface Account {
  email: string;
  password: string;
  handle: string;
  displayName: string;
}

let seq = 0;
/** Unique-ish account creds derived from a per-test timestamp + counter. */
export function freshAccount(ts: number, prefix = 'e2e'): Account {
  seq += 1;
  const tag = `${ts.toString(36)}${seq.toString(36)}`.slice(-10);
  return {
    email: `${prefix}-${ts}-${seq}@example.com`,
    password: 'longenoughpw1',
    handle: `${prefix}${tag}`.slice(0, 28),
    displayName: `E2E ${tag}`
  };
}

/** Create an account through the public signup API. Throws on non-2xx. */
export async function apiSignup(request: APIRequestContext, acc: Account): Promise<void> {
  const res = await request.post(`${BACKEND}/api/auth/signup`, {
    data: {
      email: acc.email,
      password: acc.password,
      display_name: acc.displayName,
      handle: acc.handle
    }
  });
  if (!res.ok()) {
    throw new Error(`signup failed: ${res.status()} ${await res.text()}`);
  }
}

/** Sign in through the UI form (must already be verified). */
export async function uiLogin(page: Page, acc: Account): Promise<void> {
  await page.goto(`${FRONTEND}/signin`);
  await page.fill('input[name="email"]', acc.email);
  await page.fill('input[name="password"]', acc.password);
  await page.click('button[type="submit"]');
}

/** Full path: signup via API, verify via SQL, sign in via UI. Returns creds. */
export async function signupVerifiedAndLogin(
  page: Page,
  request: APIRequestContext,
  ts: number,
  prefix = 'e2e'
): Promise<Account> {
  const acc = freshAccount(ts, prefix);
  await apiSignup(request, acc);
  verifyEmail(acc.email);
  await uiLogin(page, acc);
  return acc;
}
