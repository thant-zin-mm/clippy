import type { ApiErrorBody, HealthResponse } from './types';

export const DEFAULT_API_BASE = 'http://127.0.0.1:7817';

/** Resolve the backend base URL from build-time env, trailing-slash safe. */
export function resolveApiBase(env: { VITE_API_BASE?: string } = import.meta.env): string {
  const raw = env.VITE_API_BASE?.trim();
  if (!raw) return DEFAULT_API_BASE;
  return raw.replace(/\/+$/, '');
}

/** Error carrying the backend's `{code, message, hint}` contract (FR-21). */
export class ApiError extends Error {
  readonly code: string;
  readonly hint: string;
  readonly status: number;

  constructor(body: ApiErrorBody, status: number) {
    super(body.message);
    this.name = 'ApiError';
    this.code = body.code;
    this.hint = body.hint;
    this.status = status;
  }
}

function unreachableError(apiBase: string): Error {
  return new Error(
    `Can't reach the local backend at ${apiBase}. ` +
      'Start it with `cargo run` in `backend/` (see README), then retry.',
  );
}

export function createApiClient(apiBase: string = resolveApiBase()) {
  async function request<T>(path: string, init?: RequestInit): Promise<T> {
    let response: Response;
    try {
      response = await fetch(`${apiBase}${path}`, init);
    } catch {
      throw unreachableError(apiBase);
    }

    if (!response.ok) {
      let body: ApiErrorBody = {
        code: 'E_UNEXPECTED_STATUS',
        message: `Backend returned HTTP ${response.status}.`,
        hint: 'Check the backend logs for details.',
      };
      try {
        body = (await response.json()) as ApiErrorBody;
      } catch {
        // Non-JSON error body (e.g. proxy HTML): keep the default shape.
      }
      throw new ApiError(body, response.status);
    }

    return (await response.json()) as T;
  }

  return {
    apiBase,
    health: (signal?: AbortSignal) => request<HealthResponse>('/api/health', { signal }),
  };
}

export type ApiClient = ReturnType<typeof createApiClient>;
