import { useState } from 'preact/hooks';
import { Layout } from './components/Layout';
import { Overview } from './pages/Overview';
import { Sessions } from './pages/Sessions';
import { Metrics } from './pages/Metrics';
import './app.css';

type Page = 'overview' | 'sessions' | 'metrics';

export function App() {
  const [currentPage] = useState<Page>('overview');

  return (
    <Layout>
      {currentPage === 'overview' && <Overview />}
      {currentPage === 'sessions' && <Sessions />}
      {currentPage === 'metrics' && <Metrics />}
    </Layout>
  );
}
