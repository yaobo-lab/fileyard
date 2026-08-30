import { createContext, useContext, useEffect, ReactNode, useRef } from 'react';
import { useAuth } from './AuthContext';
import { useTenant } from './TenantContext';
import { useNavigate } from 'react-router-dom';

interface ComplianceContextType {
    // Add any exposed methods if needed
}

const ComplianceContext = createContext<ComplianceContextType | undefined>(undefined);

export function ComplianceProvider({ children }: { children: ReactNode }) {
    const { user, logout } = useAuth();
    const { currentCompany } = useTenant();
    const navigate = useNavigate();

    // Idle timeout ref
    const idleTimerRef = useRef<any>(null);
    const lastActivityRef = useRef<number>(Date.now());

    // 15 minutes in milliseconds
    const IDLE_TIMEOUT = 15 * 60 * 1000;

    const resetIdleTimer = () => {
        lastActivityRef.current = Date.now();
    };

    const checkIdle = () => {
        if (!user || !currentCompany) return;

        const mode = currentCompany.compliance_mode;
        if (mode === 'hipaa' || mode === 'soc2' || mode === 'gdpr') {
            const now = Date.now();
            if (now - lastActivityRef.current > IDLE_TIMEOUT) {
                console.log('Idle timeout reached for compliance mode:', mode);
                logout();
                navigate('/login');
                alert('You have been logged out due to inactivity for security compliance.');
            }
        }
    };

    useEffect(() => {
        // Events to track activity
        const events = ['mousedown', 'keydown', 'scroll', 'touchstart'];

        const handleActivity = () => {
            resetIdleTimer();
        };

        events.forEach(event => {
            document.addEventListener(event, handleActivity);
        });

        // Check idle status every minute
        const interval = setInterval(checkIdle, 60 * 1000);

        return () => {
            events.forEach(event => {
                document.removeEventListener(event, handleActivity);
            });
            clearInterval(interval);
        };
    }, [user, currentCompany]);

    return (
        <ComplianceContext.Provider value={{}}>
            {children}
        </ComplianceContext.Provider>
    );
}

export function useCompliance() {
    const context = useContext(ComplianceContext);
    if (context === undefined) {
        throw new Error('useCompliance must be used within a ComplianceProvider');
    }
    return context;
}
