import { DesignSystemProvider } from '@alex/ui';
import '@alex/ui/styles.css';
import React from 'react';
import ReactDOM from 'react-dom/client';
import '@alex/theme-alex/default.css';
import App from './App';
import './styles.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <DesignSystemProvider>
      <App />
    </DesignSystemProvider>
  </React.StrictMode>,
);
