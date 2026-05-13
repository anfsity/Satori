import type { SearchResponse, ApiErrorResponse } from "../types/api";

const API_BASE_URL =
  import.meta.env.VITE_SATORI_API_BASE_URL ?? "http://127.0.0.1:3000";

export async function searchSatori(query: string, limit = 10): Promise<SearchResponse> {
  const params = new URLSearchParams({
    q: query.trim(),
    limit: String(limit),
  });
  
  try {
    const response = await fetch(`${API_BASE_URL}/api/search?${params}`);
    const payload = await response.json();

    if (!response.ok) {
      throw payload as ApiErrorResponse;
    }

    return payload as SearchResponse;
  } catch (error) {
    if ((error as ApiErrorResponse).error) {
      throw error;
    }
    throw {
      error: "network_error",
      message: "无法连接到后端服务，请检查后端是否启动。",
    } as ApiErrorResponse;
  }
}

export async function checkHealth(): Promise<boolean> {
  try {
    const response = await fetch(`${API_BASE_URL}/api/health`);
    return response.ok;
  } catch {
    return false;
  }
}
