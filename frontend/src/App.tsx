import { useState, useEffect } from 'react';
import './App.css';
import { SearchBox } from './components/SearchBox';
import { SearchResultList } from './components/SearchResultList';
import { EmptyState, ErrorState } from './components/States';
import { searchSatori, checkHealth } from './api/client';
import type { ApiErrorResponse, SearchResult } from './types/api';
import heroImage from './assets/hero.png';

function getErrorMessage(error: unknown): string {
  if (
    typeof error === 'object' &&
    error !== null &&
    'message' in error &&
    typeof (error as ApiErrorResponse).message === 'string'
  ) {
    return (error as ApiErrorResponse).message;
  }

  return '搜索失败，请稍后再试。';
}

function App() {
  const [results, setResults] = useState<SearchResult[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lastQuery, setLastQuery] = useState('');
  const [isBackendUp, setIsBackendUp] = useState<boolean | null>(null);

  useEffect(() => {
    checkHealth().then(setIsBackendUp);
  }, []);

  const handleSearch = async (query: string) => {
    if (!query.trim()) return;

    setIsLoading(true);
    setError(null);
    setLastQuery(query);

    try {
      const response = await searchSatori(query);
      setResults(response.results);
      setIsBackendUp(true);
    } catch (err) {
      setError(getErrorMessage(err));
      if ((err as ApiErrorResponse).error === 'network_error') {
        setIsBackendUp(false);
      }
      setResults([]);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="app-container">
      <header className="app-header">
        <img className="app-mark" src={heroImage} alt="" aria-hidden="true" decoding="async" />
        <h1 className="app-title">Satori</h1>
        <p className="app-subtitle">中文黑话与网络梗检索工具</p>
      </header>

      <main className="app-main">
        {isBackendUp === false && (
          <div className="backend-warning">
            检查到后端服务未启动。请运行 <code>cargo run -p satori-api</code>。
          </div>
        )}

        <SearchBox onSearch={handleSearch} isLoading={isLoading} />

        {error && <ErrorState error={error} />}

        {!isLoading && !error && results.length > 0 && (
          <div className="results-summary">
            找到 {results.length} 条关于 "{lastQuery}" 的结果
          </div>
        )}

        {isLoading ? (
          <div className="loading-state">搜索中...</div>
        ) : (
          !error && (
            results.length > 0 ? (
              <SearchResultList results={results} />
            ) : (
              lastQuery && <EmptyState query={lastQuery} />
            )
          )
        )}
      </main>

      <footer className="app-footer">
        <p>&copy; 2026 Satori Project.</p>
      </footer>
    </div>
  );
}

export default App;
