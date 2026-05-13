import React from 'react';
import type { SearchResult } from '../types/api';
import './SearchResultItem.css';

interface SearchResultItemProps {
  result: SearchResult;
}

export const SearchResultItem: React.FC<SearchResultItemProps> = ({ result }) => {
  return (
    <div className="search-result-item">
      <div className="result-header">
        <h3 className="result-term">{result.term}</h3>
        <div className="result-tags">
          {result.tags.map((tag) => (
            <span key={tag} className="tag">
              {tag}
            </span>
          ))}
        </div>
      </div>
      <div className="result-plain">
        <span className="label">人话：</span>
        {result.plain}
      </div>
      <div className="result-explanation">
        <p>{result.explanation}</p>
      </div>
      {result.examples.length > 0 && (
        <div className="result-examples">
          <span className="label">例句：</span>
          <ul>
            {result.examples.map((ex, i) => (
              <li key={i}>{ex}</li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
};
