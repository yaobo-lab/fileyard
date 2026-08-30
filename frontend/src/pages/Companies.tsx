import { useState, useEffect } from 'react';
import { Plus, Search, Filter, Building2, CheckCircle, XCircle, Shield, HelpCircle, ChevronRight, Users } from 'lucide-react';
import clsx from 'clsx';
import { useNavigate } from 'react-router-dom';
import { useAuth, useAuthFetch } from '../context/AuthContext';
import { FilterModal } from '../components/FilterModal';
import { AddCompanyModal, CompanyData } from '../components/AddCompanyModal';
import { InviteUserModal, UserData } from '../components/InviteUserModal';
import { HelpPanel } from '../components/HelpPanel';

interface Company {
    id: string;
    name: string;
    domain: string;
    plan: string;
    status: string;
    compliance_mode: string;
    user_count?: number;
    storage_used_bytes?: number;
    created_at: string;
    storage_quota_bytes?: number;
}

const statusFilterOptions = [
    { label: 'Active', value: 'active' },
    { label: 'Suspended', value: 'suspended' },
    { label: 'Trial', value: 'trial' },
];

export function Companies() {
    const [companies, setCompanies] = useState<Company[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [searchTerm, setSearchTerm] = useState('');
    const [isFilterOpen, setIsFilterOpen] = useState(false);
    const [isAddModalOpen, setIsAddModalOpen] = useState(false);
    const [isInviteModalOpen, setIsInviteModalOpen] = useState(false);
    const [selectedCompanyForInvite, setSelectedCompanyForInvite] = useState<Company | null>(null);
    const [isHelpOpen, setIsHelpOpen] = useState(false);
    const [filters, setFilters] = useState<any>({});

    const { user, hasPermission } = useAuth();
    const authFetch = useAuthFetch();
    const navigate = useNavigate();

    // Check if user can manage companies (either SuperAdmin or has tenants.manage permission)
    const canManageCompanies = hasPermission('tenants.manage');
    const isSuperAdmin = user?.role === 'SuperAdmin';

    useEffect(() => {
        if (canManageCompanies) {
            fetchCompanies();
        }
    }, [filters, canManageCompanies]);

    const fetchCompanies = async () => {
        try {
            setIsLoading(true);

            const params = new URLSearchParams();
            if (filters.status) params.append('status', filters.status);
            if (filters.search) params.append('search', filters.search);

            // SuperAdmin sees all companies, others see only their accessible companies
            const endpoint = isSuperAdmin 
                ? `/api/tenants?${params.toString()}`
                : `/api/tenants/accessible?${params.toString()}`;

            const response = await authFetch(endpoint);

            if (!response.ok) throw new Error('Failed to fetch companies');

            const data = await response.json();
            setCompanies(data);
        } catch (error) {
            console.error('Error fetching companies:', error);
        } finally {
            setIsLoading(false);
        }
    };

    const handleAddCompany = async (data: CompanyData) => {
        const response = await authFetch('/api/tenants', {
            method: 'POST',
            body: JSON.stringify({
                ...data,
                plan: 'enterprise' // Default plan
            }),
        });

        if (!response.ok) {
            throw new Error('Failed to add company');
        }

        fetchCompanies();
    };

    const handleInviteUser = async (data: UserData) => {
        if (!selectedCompanyForInvite) return;

        const response = await authFetch('/api/users', {
            method: 'POST',
            body: JSON.stringify({
                ...data,
                tenant_id: selectedCompanyForInvite.id
            }),
        });

        if (!response.ok) {
            throw new Error('Failed to invite user');
        }

        fetchCompanies(); // Update user counts
        setIsInviteModalOpen(false);
        setSelectedCompanyForInvite(null);
    };

    const filteredCompanies = searchTerm
        ? companies.filter(c => c.name.toLowerCase().includes(searchTerm.toLowerCase()) || c.domain.toLowerCase().includes(searchTerm.toLowerCase()))
        : companies;

    if (!canManageCompanies) {
        return (
            <div className="flex items-center justify-center h-96">
                <div className="text-center">
                    <Shield className="mx-auto h-10 w-10 text-muted-foreground/60" />
                    <h3 className="mt-2 text-sm font-semibold text-foreground">Access Restricted</h3>
                    <p className="mt-1 text-xs text-muted-foreground">You don't have permission to manage companies.</p>
                </div>
            </div>
        );
    }

    return (
        <div className="space-y-4 sm:space-y-6">
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 sm:gap-4">
                <div>
                    <h1 className="text-xl sm:text-2xl font-bold text-foreground">Companies</h1>
                    <p className="text-xs sm:text-sm text-muted-foreground mt-0.5 sm:mt-1">
                        {isSuperAdmin 
                            ? 'Manage all companies and tenant organizations.' 
                            : 'Manage companies you have access to.'}
                    </p>
                </div>
                <div className="flex items-center gap-2 sm:gap-3">
                    <button
                        onClick={() => setIsHelpOpen(true)}
                        className="px-3 sm:px-4 py-1.5 bg-card border border-border text-foreground rounded-lg text-sm font-medium hover:bg-muted transition-colors flex items-center"
                    >
                        <HelpCircle className="w-4 h-4 sm:mr-1.5" />
                        <span className="hidden sm:inline">Help & Roles</span>
                    </button>
                    {isSuperAdmin && (
                        <button
                            onClick={() => setIsAddModalOpen(true)}
                            className="px-3 sm:px-4 py-1.5 bg-foreground text-background rounded-lg text-sm font-medium hover:bg-foreground/90 flex items-center transition-all"
                        >
                            <Plus className="w-4 h-4 sm:mr-1.5" />
                            <span className="hidden sm:inline">Add Company</span>
                        </button>
                    )}
                </div>
            </div>

            <div className="bg-card border border-border rounded-xl overflow-hidden transition-colors">
                <div className="p-3 sm:p-4 border-b border-border flex items-center justify-between gap-3 bg-muted/20">
                    <div className="relative flex-1 max-w-md">
                        <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                            <Search className="h-4 w-4 text-muted-foreground/60" />
                        </div>
                        <input
                            type="text"
                            className="block w-full pl-9 pr-3 py-1.5 border border-border rounded-lg bg-muted/40 placeholder-muted-foreground/60 text-foreground focus:outline-none focus:ring-2 focus:ring-ring text-sm transition-all"
                            placeholder="Search companies..."
                            value={searchTerm}
                            onChange={(e) => setSearchTerm(e.target.value)}
                        />
                    </div>
                    <button
                        onClick={() => setIsFilterOpen(true)}
                        className="px-3 py-1.5 bg-card border border-border rounded-lg text-sm font-medium text-foreground hover:bg-muted flex items-center transition-colors flex-shrink-0">
                        <Filter className="w-4 h-4 sm:mr-1.5" />
                        <span className="hidden sm:inline">Filters</span>
                        {filters.status && <span className="ml-2 w-2 h-2 bg-foreground rounded-full"></span>}
                    </button>
                </div>

                <div>
                    {isLoading ? (
                        <div className="p-12 text-center">
                            <div className="animate-spin rounded-full h-6 w-6 border-2 border-foreground border-t-transparent mx-auto mb-4"></div>
                            <p className="text-sm text-muted-foreground">Loading companies...</p>
                        </div>
                    ) : filteredCompanies.length === 0 ? (
                        <div className="p-12 text-center text-muted-foreground">
                            <Building2 className="w-10 h-10 mx-auto text-muted-foreground/40 mb-3" />
                            <p className="text-sm">No companies found</p>
                        </div>
                    ) : (
                        <>
                            {/* Mobile: Card view */}
                            <div className="sm:hidden divide-y divide-border">
                                {filteredCompanies.map((company) => (
                                    <div
                                        key={company.id}
                                        onClick={() => navigate(`/companies/${encodeURIComponent(company.name)}`)}
                                        className="p-4 hover:bg-muted/30 transition-colors cursor-pointer active:bg-muted"
                                    >
                                        <div className="flex items-start justify-between gap-3">
                                            <div className="flex items-center gap-3 min-w-0 flex-1">
                                                <div className="h-9 w-9 rounded-lg bg-muted flex items-center justify-center text-foreground flex-shrink-0">
                                                    <Building2 className="w-4 h-4" />
                                                </div>
                                                <div className="min-w-0 flex-1">
                                                    <p className="font-semibold text-foreground truncate">{company.name}</p>
                                                    <p className="text-xs text-muted-foreground truncate">{company.domain}</p>
                                                </div>
                                            </div>
                                            <ChevronRight className="w-5 h-5 text-muted-foreground/60 flex-shrink-0" />
                                        </div>
                                        <div className="flex items-center gap-3 mt-3 flex-wrap">
                                            <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-semibold bg-muted text-foreground">
                                                {company.compliance_mode ? company.compliance_mode.toUpperCase() : 'STANDARD'}
                                            </span>
                                            <span className="inline-flex items-center text-xs text-muted-foreground">
                                                <Users className="w-3.5 h-3.5 mr-1" />
                                                {company.user_count || 0} users
                                            </span>
                                            <span className={clsx(
                                                "inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium",
                                                company.status === 'active'
                                                    ? "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400"
                                                    : company.status === 'suspended'
                                                        ? "bg-destructive/10 text-destructive"
                                                        : "bg-amber-500/10 text-amber-600 dark:text-amber-400"
                                            )}>
                                                {company.status === 'active' ? <CheckCircle className="w-3 h-3 mr-1" /> : <XCircle className="w-3 h-3 mr-1" />}
                                                {company.status.charAt(0).toUpperCase() + company.status.slice(1)}
                                            </span>
                                        </div>
                                        <button
                                            onClick={(e) => {
                                                e.stopPropagation();
                                                setSelectedCompanyForInvite(company);
                                                setIsInviteModalOpen(true);
                                            }}
                                            className="mt-3 w-full py-1.5 text-xs font-semibold text-foreground bg-muted hover:bg-muted/80 rounded-lg transition-colors"
                                        >
                                            Add User
                                        </button>
                                    </div>
                                ))}
                            </div>

                            {/* Desktop: Table view */}
                            <table className="hidden sm:table min-w-full divide-y divide-border">
                                <thead className="bg-muted/50">
                                    <tr>
                                        <th className="px-5 py-3 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider">Company</th>
                                        <th className="px-5 py-3 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider">Compliance</th>
                                        <th className="px-5 py-3 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider">Users</th>
                                        <th className="px-5 py-3 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider">Status</th>
                                        <th className="relative px-5 py-3"><span className="sr-only">Actions</span></th>
                                    </tr>
                                </thead>
                                <tbody className="divide-y divide-border">
                                    {filteredCompanies.map((company) => (
                                        <tr
                                            key={company.id}
                                            onClick={() => navigate(`/companies/${encodeURIComponent(company.name)}`)}
                                            className="hover:bg-muted/30 transition-colors cursor-pointer group"
                                        >
                                            <td className="px-5 py-3.5 whitespace-nowrap">
                                                <div className="flex items-center">
                                                    <div className="h-9 w-9 rounded-lg bg-muted flex items-center justify-center text-foreground">
                                                        <Building2 className="w-4 h-4" />
                                                    </div>
                                                    <div className="ml-3">
                                                        <div className="text-sm font-semibold text-foreground group-hover:underline transition-colors">
                                                            {company.name}
                                                        </div>
                                                        <div className="text-xs text-muted-foreground mt-0.5">{company.domain}</div>
                                                    </div>
                                                </div>
                                            </td>
                                            <td className="px-5 py-3.5 whitespace-nowrap text-sm text-muted-foreground">
                                                {company.compliance_mode ? (
                                                    <span className="inline-flex items-center px-2.5 py-0.5 rounded text-xs font-semibold bg-muted text-foreground">
                                                        {company.compliance_mode.toUpperCase()}
                                                    </span>
                                                ) : (
                                                    <span className="text-muted-foreground/60">Standard</span>
                                                )}
                                            </td>
                                            <td className="px-5 py-3.5 whitespace-nowrap text-sm text-muted-foreground">
                                                <div className="flex items-center">
                                                    <Users className="w-4 h-4 mr-1.5 text-muted-foreground/60" />
                                                    {company.user_count || 0}
                                                </div>
                                            </td>
                                            <td className="px-5 py-3.5 whitespace-nowrap">
                                                <span className={clsx(
                                                    "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-semibold",
                                                    company.status === 'active'
                                                        ? "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400"
                                                        : company.status === 'suspended'
                                                            ? "bg-destructive/10 text-destructive"
                                                            : "bg-amber-500/10 text-amber-600 dark:text-amber-400"
                                                )}>
                                                    {company.status === 'active' ? <CheckCircle className="w-3 h-3 mr-1" /> : <XCircle className="w-3 h-3 mr-1" />}
                                                    {company.status.charAt(0).toUpperCase() + company.status.slice(1)}
                                                </span>
                                            </td>
                                            <td className="px-5 py-3.5 whitespace-nowrap text-right text-sm font-medium">
                                                <div className="flex items-center justify-end gap-3">
                                                    <button
                                                        onClick={(e) => {
                                                            e.stopPropagation();
                                                            setSelectedCompanyForInvite(company);
                                                            setIsInviteModalOpen(true);
                                                        }}
                                                        className="text-foreground hover:underline px-3 py-1 rounded-md hover:bg-muted transition-colors text-xs font-semibold"
                                                    >
                                                        Add User
                                                    </button>
                                                    <ChevronRight className="w-5 h-5 text-muted-foreground/60 group-hover:text-foreground transition-colors" />
                                                </div>
                                            </td>
                                        </tr>
                                    ))}
                                </tbody>
                            </table>
                        </>
                    )}
                </div>
            </div>

            <FilterModal
                isOpen={isFilterOpen}
                onClose={() => setIsFilterOpen(false)}
                onApply={setFilters}
                config={{
                    status: statusFilterOptions,
                    search: true,
                }}
                initialValues={filters}
            />
            <AddCompanyModal
                isOpen={isAddModalOpen}
                onClose={() => setIsAddModalOpen(false)}
                onSubmit={handleAddCompany}
            />
            <InviteUserModal
                isOpen={isInviteModalOpen}
                onClose={() => {
                    setIsInviteModalOpen(false);
                    setSelectedCompanyForInvite(null);
                }}
                onSubmit={handleInviteUser}
                targetTenantId={selectedCompanyForInvite?.id}
            />
            <HelpPanel
                isOpen={isHelpOpen}
                onClose={() => setIsHelpOpen(false)}
            />
        </div>
    );
}
