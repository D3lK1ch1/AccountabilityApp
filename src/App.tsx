import { useAppStore } from './store/useAppStore';
import { useEffect } from 'react';
import { ConsentModal } from './components/ConsentModal';
import { Widget } from './components/Widget';
import './App.css';

function App() {
  const { consentGiven, checkConsent } = useAppStore();

  useEffect(() => {
    checkConsent();
  }, []);

  if (consentGiven === null) {
    return <div />;
  }
  if(!consentGiven) {
    return <ConsentModal />;
  }
  return <Widget />;
}

export default App;
