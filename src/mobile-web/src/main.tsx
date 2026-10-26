import React from 'react';
import ReactDOM from 'react-dom/client';
import './styles/reset.scss';
import '@alex/theme-alex/default.css';
import '@alex/ui/mobile.css';
import App from './App';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
