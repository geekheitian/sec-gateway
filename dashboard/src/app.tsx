import { AppProvider, useNavigation } from './Navigation';
import { Layout } from './components/Layout';
import { Overview } from './pages/Overview';
import { Sessions } from './pages/Sessions';
import { Metrics } from './pages/Metrics';
import './app.css';

function AppContent() {
  const { currentPage } = useNavigation();

  return (
    <Layout>
      {currentPage === 'overview' && <Overview />}
      {currentPage === 'sessions' && <Sessions />}
      {currentPage === 'metrics' && <Metrics />}
    </Layout>
  );
}

export function App() {
  return (
    <AppProvider>
      <AppContent />
    </AppProvider>
  );
}
