import React from 'react';
import type { SearchResult } from '../types/api';
import { SearchResultItem } from './SearchResultItem';

interface SearchResultListProps {
  results: SearchResult[];
}

export const SearchResultList: React.FC<SearchResultListProps> = ({ results }) => {
  return (
    <div className="search-result-list">
      {results.map((result) => (
        <SearchResultItem key={result.id} result={result} />
      ))}
    </div>
  );
};
