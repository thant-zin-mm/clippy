import { afterEach, describe, expect, it, vi } from 'vitest';

import { ApiError, createApiClient, DEFAULT_API_BASE, resolveApiBase } from './client';

describe('resolveApiBase', () => {
  it('defaults when unset, empty, or blank', () => {
    expect(resolveApiBase({})).toBe(DEFAULT_API_BASE);
    expect(resolveApiBase({ VITE_API_BASE: '' })).toBe(DEFAULT_API_BASE);
    expect(resolveApiBase({ VITE_API_BASE: '   ' })).toBe(DEFAULT_API_BASE);
  });

  it('strips trailing slashes', () => {
    expect(resolveApiBase({ VITE_API_BASE: 'http://127.0.0.1:9000/' })).toBe(
      'http://127.0.0.1:9000',
    );
    expect(resolveApiBase({ VITE_API_BASE: 'http://127.0.0.1:9000//' })).toBe(
      'http://127.0.0.1:9000',
    );
  });

  it('uses an explicit base as-is (no trailing slash)', () => {
    expect(resolveApiBase({ VITE_API_BASE: 'http://localhost:8080' })).toBe(
      'http://localhost:8080',
    );
  });
});

describe('createApiClient().health', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('returns the parsed health payload', async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ ok: true, version: '0.1.0' }), {
        status: 200,
        headers: { 'content-type': 'application/json' },
      }),
    );
    vi.stubGlobal('fetch', fetchMock);

    const client = createApiClient('http://api.test');
    await expect(client.health()).resolves.toEqual({ ok: true, version: '0.1.0' });
    expect(fetchMock).toHaveBeenCalledWith('http://api.test/api/health', { signal: undefined });
  });

  it('maps backend error bodies to ApiError', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValue(
        new Response(
          JSON.stringify({ code: 'E_UNSUPPORTED_FORMAT', message: 'bad file', hint: 'use mp3' }),
          { status: 400, headers: { 'content-type': 'application/json' } },
        ),
      );
    vi.stubGlobal('fetch', fetchMock);

    const client = createApiClient('http://api.test');
    const failure = client.health();
    await expect(failure).rejects.toBeInstanceOf(ApiError);
    await expect(failure).rejects.toMatchObject({
      code: 'E_UNSUPPORTED_FORMAT',
      status: 400,
      hint: 'use mp3',
    });
  });

  it('falls back to a default error shape for non-JSON error bodies', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response('<html>502</html>', { status: 502 }));
    vi.stubGlobal('fetch', fetchMock);

    const client = createApiClient('http://api.test');
    await expect(client.health()).rejects.toMatchObject({
      code: 'E_UNEXPECTED_STATUS',
      status: 502,
    });
  });

  it('surfaces an unreachable-backend message when fetch rejects', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new TypeError('Failed to fetch')));

    const client = createApiClient('http://127.0.0.1:7817');
    await expect(client.health()).rejects.toThrow(/Can't reach the local backend/);
  });
});
