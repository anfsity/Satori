import React from 'react';
import './States.css';

interface EmptyStateProps {
  query: string;
}

export const EmptyState: React.FC<EmptyStateProps> = ({ query }) => {
  return (
    <div className="empty-state">
      <p>没有找到与 "{query}" 相关的匹配词条。</p>
      <p className="empty-state-hint">建议换一种说法或缩短查询关键词。</p>
    </div>
  );
};

interface ErrorStateProps {
  error: string;
}

export const ErrorState: React.FC<ErrorStateProps> = ({ error }) => {
  return (
    <div className="error-state">
      <p>
        <strong>出错了：</strong>
        {error}
      </p>
    </div>
  );
};
