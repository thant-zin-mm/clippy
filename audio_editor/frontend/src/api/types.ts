/** Response DTOs mirroring the API contract (docs/architecture.md §6). */

export interface HealthResponse {
  ok: boolean;
  version: string;
}

export interface ApiErrorBody {
  code: string;
  message: string;
  hint: string;
}
