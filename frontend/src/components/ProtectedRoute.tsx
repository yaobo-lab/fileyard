import { Navigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';

interface ProtectedRouteProps {
    children: React.ReactNode;
}

export function ProtectedRoute({ children }: ProtectedRouteProps) {
    const { user, isLoading } = useAuth();

    if (isLoading) {
        return <div className="flex items-center justify-center h-screen">Loading...</div>;
    }

    if (!user) {
        return <Navigate to="/login" replace />;
    }

    return <>{children}</>;
}

// Role-based route protection
interface RoleProtectedRouteProps {
    children: React.ReactNode;
    allowedRoles: string[];
    redirectTo?: string;
}

export function RoleProtectedRoute({ children, allowedRoles, redirectTo = '/files' }: RoleProtectedRouteProps) {
    const { user, isLoading } = useAuth();

    if (isLoading) {
        return <div className="flex items-center justify-center h-screen">Loading...</div>;
    }

    if (!user) {
        return <Navigate to="/login" replace />;
    }

    // Check if user's role is in the allowed list
    if (!allowedRoles.includes(user.role)) {
        return <Navigate to={redirectTo} replace />;
    }

    return <>{children}</>;
}

// Convenience components for common role checks
export function AdminRoute({ children }: { children: React.ReactNode }) {
    return (
        <RoleProtectedRoute allowedRoles={['SuperAdmin', 'Admin']}>
            {children}
        </RoleProtectedRoute>
    );
}

export function SuperAdminRoute({ children }: { children: React.ReactNode }) {
    return (
        <RoleProtectedRoute allowedRoles={['SuperAdmin']}>
            {children}
        </RoleProtectedRoute>
    );
}

// Permission-based route protection
interface PermissionProtectedRouteProps {
    children: React.ReactNode;
    permission: string;
    redirectTo?: string;
}

export function PermissionProtectedRoute({ 
    children, 
    permission, 
    redirectTo = '/files' 
}: PermissionProtectedRouteProps) {
    const { user, hasPermission, isLoading } = useAuth();

    if (isLoading) {
        return <div className="flex items-center justify-center h-screen">Loading...</div>;
    }

    if (!user) {
        return <Navigate to="/login" replace />;
    }

    // Check if user has the required permission
    if (!hasPermission(permission)) {
        return <Navigate to={redirectTo} replace />;
    }

    return <>{children}</>;
}
