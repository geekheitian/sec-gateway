import { createContext } from 'preact';
import { useContext, useState } from 'preact/hooks';
import type { ComponentChildren } from 'preact';

type Page = 'overview' | 'sessions' | 'metrics';

interface NavigationContextValue {
  currentPage: Page;
  navigate: (page: Page) => void;
}

const NavigationContext = createContext<NavigationContextValue>({
  currentPage: 'overview',
  navigate: () => {},
});

export function useNavigation() {
  return useContext(NavigationContext);
}

interface AppProviderProps {
  children: ComponentChildren;
}

export function AppProvider({ children }: AppProviderProps) {
  const [currentPage, setCurrentPage] = useState<Page>('overview');

  const navigate = (page: Page) => {
    setCurrentPage(page);
  };

  return (
    <NavigationContext.Provider value={{ currentPage, navigate }}>
      {children}
    </NavigationContext.Provider>
  );
}
