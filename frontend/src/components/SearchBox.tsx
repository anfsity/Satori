import React, { useState } from 'react';
import './SearchBox.css';

interface SearchBoxProps {
  onSearch: (query: string) => void;
  isLoading: boolean;
}

const EXAMPLES = ['大家先统一想法', '心态崩了', '故意搞点节目效果'];

export const SearchBox: React.FC<SearchBoxProps> = ({ onSearch, isLoading }) => {
  const [query, setQuery] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (query.trim()) {
      onSearch(query.trim());
    }
  };

  return (
    <div className="search-box">
      <form onSubmit={handleSubmit} className="search-form">
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="输入白话、黑话或场景描述..."
          className="search-input"
          disabled={isLoading}
          autoFocus
        />
        <button type="submit" className="search-button" disabled={isLoading || !query.trim()}>
          {isLoading ? '搜索中...' : '搜索'}
        </button>
      </form>
      <div className="examples">
        <span>示例：</span>
        {EXAMPLES.map((ex) => (
          <button
            key={ex}
            className="example-link"
            onClick={() => {
              setQuery(ex);
              onSearch(ex);
            }}
            disabled={isLoading}
          >
            {ex}
          </button>
        ))}
      </div>
    </div>
  );
};
