export interface HealthResponse {
  status: "ok";
}

export interface SearchResponse {
  query: string;
  results: SearchResult[];
}

export interface SearchResult {
  id: string;
  term: string;
  plain: string;
  explanation: string;
  examples: string[];
  tags: string[];
  score: number;
}

export interface ApiErrorResponse {
  error: "invalid_query" | "invalid_limit" | "search_failed" | string;
  message: string;
}
