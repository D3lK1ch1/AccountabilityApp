import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useAppStore } from '../store/useAppStore';
import './Widget.css';

export function ConsentModal() {
    const{
        checkConsent
    } = useAppStore();

    const handleAgree = async () => {
        await invoke('set_setting', { key: 'consent_given', value: 'true' });
        await checkConsent();
    };

    const handleDecline = async () => {
        useAppStore.setState({ consentGiven: false });
    };

    const handleDragStart = (e: React.MouseEvent) => {
        if ((e.target as HTMLElement).closest('button')) return;
          getCurrentWindow().startDragging().catch(console.error);
      };

      return (
        <div className = 'widget'>
            <div className="widget-header"
            onMouseDown ={handleDragStart}
            >
                <div className = "header-left">
                    <span className = "title">Terms & Conditions</span>
                </div>
                <div className = "widget-content">
                    <p>We use tracking data to provide insights and improve your experience. That means the app names and tab titles used are recorded (for example, social media tracking for future blocking features)</p> 
                    <p> Note that the data will stay in your device for your own use and not be uploaded anywhere else by third-parties. To clear any data (Settings - Clear Data)</p> 
                    <p>Your privacy is important to us, and we do not share your data with third parties.</p>
                    <p> The app can be used once you agree to these conditions</p>
                    <button onClick = {handleAgree}>I Agree</button>
                    <button onClick = {handleDecline}>I Decline</button>
                </div>
            </div>
        </div>
      );
}

export default ConsentModal;

