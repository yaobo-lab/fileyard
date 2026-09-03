import { NavLink, Outlet, useNavigate } from 'react-router-dom';
import { Settings, Image, Shield, ShieldCheck, Building2, ArrowRight, BookOpen, Wrench, Mail, Keyboard, Globe, HardDrive } from 'lucide-react';
import { useAuth } from '../../context/AuthContext';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import clsx from 'clsx';

const TABS = [
    { id: 'general', label: 'General', path: '/settings/general', icon: Settings },
    { id: 'branding', label: 'Branding', path: '/settings/branding', icon: Image },
    { id: 'pages', label: 'Pages', path: '/settings/pages', icon: BookOpen },
    { id: 'email-templates', label: 'Email Templates', path: '/settings/email-templates', icon: Mail },
    { id: 'shortcuts', label: 'Shortcuts', path: '/settings/shortcuts', icon: Keyboard },
    { id: 'system', label: 'System', path: '/settings/system', icon: Wrench },
    { id: 'virus-scan', label: 'Virus Scan', path: '/settings/virus-scan', icon: ShieldCheck },
    { id: 'sso', label: 'SSO', path: '/settings/sso', icon: Globe },
    { id: 'backup', label: 'Backup', path: '/settings/backup', icon: HardDrive },
    { id: 'admin', label: 'Administration', path: '/settings/admin', icon: Shield },
];

export function SettingsLayout() {
    const { user, tenant } = useAuth();
    const navigate = useNavigate();

    // Non-SuperAdmin users get redirected to Company Details (per-tenant settings)
    if (!user || user.role !== 'SuperAdmin') {
        return (
            <div className="max-w-xl mx-auto py-12 px-4">
                <Card className="text-center shadow-sm">
                    <CardHeader className="pb-4 flex flex-col items-center">
                        <div className="size-12 rounded-full bg-primary/10 flex items-center justify-center mb-2">
                            <Building2 className="size-6 text-primary" />
                        </div>
                        <CardTitle className="text-xl font-bold">Company Settings</CardTitle>
                        <CardDescription className="max-w-sm mx-auto">
                            Company settings, departments, and compliance configurations are managed directly on the Company Details page.
                        </CardDescription>
                    </CardHeader>
                    <CardContent>
                        <Button
                            onClick={() => navigate(`/companies/${encodeURIComponent(tenant?.name || '')}`)}
                            className="gap-2"
                        >
                            <span>Go to Company Details</span>
                            <ArrowRight className="size-4" />
                        </Button>
                    </CardContent>
                </Card>
            </div>
        );
    }

    return (
        <div className="space-y-6">
            {/* Header */}
            <div>
                <h1 className="text-2xl font-bold tracking-tight text-foreground">Global Settings</h1>
                <p className="text-sm text-muted-foreground mt-1">
                    Configure system-wide preferences, security, integrations, and administration for all organizations.
                </p>
            </div>

            <Separator />

            {/* Tab Navigation */}
            <div className="flex flex-col space-y-6">
                <div className="overflow-x-auto pb-1">
                    <nav className="flex items-center gap-1 border-b border-border min-w-max pb-px">
                        {TABS.map((tab) => {
                            const Icon = tab.icon;
                            return (
                                <NavLink
                                    key={tab.id}
                                    to={tab.path}
                                    className={({ isActive }) =>
                                        clsx(
                                            "flex items-center gap-2 px-3.5 py-2 text-sm font-medium border-b-2 transition-all whitespace-nowrap rounded-t-md",
                                            isActive
                                                ? "border-primary text-foreground font-semibold bg-muted/40"
                                                : "border-transparent text-muted-foreground hover:text-foreground hover:bg-muted/20"
                                        )
                                    }
                                >
                                    <Icon className="size-4" />
                                    <span>{tab.label}</span>
                                </NavLink>
                            );
                        })}
                    </nav>
                </div>

                {/* Content Area */}
                <div className="min-w-0">
                    <Outlet />
                </div>
            </div>
        </div>
    );
}
