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
                    <Shield className="mx-auto h-12 w-12 text-gray-400 dark:text-gray-500" />
                    <h3 className="mt-2 text-sm font-medium text-gray-900 dark:text-white">Access Restricted</h3>
                    <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">You don't have permission to manage companies.</p>
                </div>
            </div>
        );
    }

    return (
        <div className="space-y-4 sm:space-y-6">
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 sm:gap-4">
                <div>
                    <h1 className="text-xl sm:text-2xl font-bold text-gray-900 dark:text-white">Companies</h1>
                    <p className="text-xs sm:text-sm text-gray-500 dark:text-gray-400 mt-0.5 sm:mt-1">
                        {isSuperAdmin 
                            ? 'Manage all companies and tenant organizations.' 
                            : 'Manage companies you have access to.'}
                    </p>
                </div>
                <div className="flex items-center gap-2 sm:gap-3">
                    <button
                        onClick={() => setIsHelpOpen(true)}
                        className="px-3 sm:px-4 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-200 rounded-lg text-sm font-medium hover:bg-gray-50 dark:hover:bg-gray-700 shadow-sm flex items-center transition-colors"
                    >
                        <HelpCircle className="w-4 h-4 sm:mr-2" />
                        <span className="hidden sm:inline">Help & Roles</span>
                    </button>
                    {isSuperAdmin && (
                        <button
                            onClick={() => setIsAddModalOpen(true)}
                            className="px-3 sm:px-4 py-2 bg-primary-600 text-white rounded-lg text-sm font-medium hover:bg-primary-700 shadow-sm flex items-center transition-all hover:shadow-md"
                        >
                            <Plus className="w-4 h-4 sm:mr-2" />
                            <span className="hidden sm:inline">Add Company</span>
                        </button>
                    )}
                </div>
            </div>

            <div className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-xl shadow-sm overflow-hidden transition-colors">
                <div className="p-3 sm:p-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between gap-3 bg-gray-50/50 dark:bg-gray-900/20">
                    <div className="relative flex-1 max-w-md">
                        <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                            <Search className="h-4 w-4 text-gray-400 dark:text-gray-500" />
                        </div>
                        <input
                            type="text"
                            className="block w-full pl-10 pr-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg leading-5 bg-white dark:bg-gray-700 placeholder-gray-500 dark:placeholder-gray-400 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent text-sm transition-all"
                            placeholder="Search companies..."
                            value={searchTerm}
                            onChange={(e) => setSearchTerm(e.target.value)}
                        />
                    </div>
                    <button
                        onClick={() => setIsFilterOpen(true)}
                        className="px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-sm font-medium text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-600 flex items-center transition-colors flex-shrink-0">
                        <Filter className="w-4 h-4 sm:mr-2 text-gray-500 dark:text-gray-400" />
                        <span className="hidden sm:inline">Filters</span>
                        {filters.status && <span className="ml-2 w-2 h-2 bg-primary-500 rounded-full"></span>}
                    </button>
                </div>

                <div>
                    {isLoading ? (
                        <div className="p-12 text-center">
                            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600 mx-auto mb-4"></div>
                            <p className="text-gray-500 dark:text-gray-400">Loading companies...</p>
                        </div>
                    ) : filteredCompanies.length === 0 ? (
                        <div className="p-12 text-center text-gray-500 dark:text-gray-400">
                            <Building2 className="w-12 h-12 mx-auto text-gray-300 dark:text-gray-600 mb-3" />
                            <p>No companies found</p>
                        </div>
                    ) : (
                        <>
                            {/* Mobile: Card view */}
                            <div className="sm:hidden divide-y divide-gray-200 dark:divide-gray-700">
                                {filteredCompanies.map((company) => (
                                    <div
                                        key={company.id}
                                        onClick={() => navigate(`/companies/${encodeURIComponent(company.name)}`)}
                                        className="p-4 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors cursor-pointer active:bg-gray-100 dark:active:bg-gray-700"
                                    >
                                        <div className="flex items-start justify-between gap-3">
                                            <div className="flex items-center gap-3 min-w-0 flex-1">
                                                <div className="h-10 w-10 rounded-lg bg-gradient-to-br from-primary-100 to-primary-200 dark:from-primary-900/40 dark:to-primary-800/40 flex items-center justify-center text-primary-700 dark:text-primary-400 shadow-sm flex-shrink-0">
                                                    <Building2 className="w-5 h-5" />
                                                </div>
                                                <div className="min-w-0 flex-1">
                                                    <p className="font-medium text-gray-900 dark:text-white truncate">{company.name}</p>
                                                    <p className="text-sm text-gray-500 dark:text-gray-400 truncate">{company.domain}</p>
                                                </div>
                                            </div>
                                            <ChevronRight className="w-5 h-5 text-gray-400 flex-shrink-0" />
                                        </div>
                                        <div className="flex items-center gap-3 mt-3 flex-wrap">
                                            <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-300">
                                                {company.compliance_mode ? company.compliance_mode.toUpperCase() : 'STANDARD'}
                                            </span>
                                            <span className="inline-flex items-center text-xs text-gray-500 dark:text-gray-400">
                                                <Users className="w-3.5 h-3.5 mr-1" />
                                                {company.user_count || 0} users
                                            </span>
                                            <span className={clsx(
                                                "inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium",
                                                company.status === 'active'
                                                    ? "bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-300"
                                                    : company.status === 'suspended'
                                                        ? "bg-red-100 dark:bg-red-900/30 text-red-800 dark:text-red-300"
                                                        : "bg-yellow-100 dark:bg-yellow-900/30 text-yellow-800 dark:text-yellow-300"
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
                                            className="mt-3 w-full py-2 text-sm font-medium text-primary-600 dark:text-primary-400 bg-primary-50 dark:bg-primary-900/20 rounded-lg hover:bg-primary-100 dark:hover:bg-primary-900/30 transition-colors"
                                        >
                                            Add User
                                        </button>
                                    </div>
                                ))}
                            </div>

                            {/* Desktop: Table view */}
                            <table className="hidden sm:table min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                                <thead className="bg-gray-50 dark:bg-gray-700/50">
                                    <tr>
                                        <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Company</th>
                                        <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Compliance</th>
                                        <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Users</th>
                                        <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Status</th>
                                        <th className="relative px-6 py-3"><span className="sr-only">Actions</span></th>
                                    </tr>
                                </thead>
                                <tbody className="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
                                    {filteredCompanies.map((company) => (
                                        <tr
                                            key={company.id}
                                            onClick={() => navigate(`/companies/${encodeURIComponent(company.name)}`)}
                                            className="hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors cursor-pointer group"
                                        >
                                            <td className="px-6 py-4 whitespace-nowrap">
                                                <div className="flex items-center">
                                                    <div className="h-10 w-10 rounded-lg bg-gradient-to-br from-primary-100 to-primary-200 dark:from-primary-900/40 dark:to-primary-800/40 flex items-center justify-center text-primary-700 dark:text-primary-400 shadow-sm">
                                                        <Building2 className="w-5 h-5" />
                                                    </div>
                                                    <div className="ml-4">
                                                        <div className="text-sm font-medium text-gray-900 dark:text-white group-hover:text-primary-600 dark:group-hover:text-primary-400 transition-colors">
                                                            {company.name}
                                                        </div>
                                                        <div className="text-sm text-gray-500 dark:text-gray-400">{company.domain}</div>
                                                    </div>
                                                </div>
                                            </td>
                                            <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                {company.compliance_mode ? (
                                                    <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-300">
                                                        {company.compliance_mode.toUpperCase()}
                                                    </span>
                                                ) : (
                                                    <span className="text-gray-400">Standard</span>
                                                )}
                                            </td>
                                            <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                <div className="flex items-center">
                                                    <Users className="w-4 h-4 mr-1.5 text-gray-400" />
                                                    {company.user_count || 0}
                                                </div>
                                            </td>
                                            <td className="px-6 py-4 whitespace-nowrap">
                                                <span className={clsx(
                                                    "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium",
                                                    company.status === 'active'
                                                        ? "bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-300"
                                                        : company.status === 'suspended'
                                                            ? "bg-red-100 dark:bg-red-900/30 text-red-800 dark:text-red-300"
                                                            : "bg-yellow-100 dark:bg-yellow-900/30 text-yellow-800 dark:text-yellow-300"
                                                )}>
                                                    {company.status === 'active' ? <CheckCircle className="w-3 h-3 mr-1" /> : <XCircle className="w-3 h-3 mr-1" />}
                                                    {company.status.charAt(0).toUpperCase() + company.status.slice(1)}
                                                </span>
                                            </td>
                                            <td className="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                                                <div className="flex items-center justify-end gap-3">
                                                    <button
                                                        onClick={(e) => {
                                                            e.stopPropagation();
                                                            setSelectedCompanyForInvite(company);
                                                            setIsInviteModalOpen(true);
                                                        }}
                                                        className="text-primary-600 hover:text-primary-900 dark:text-primary-400 dark:hover:text-primary-300 px-3 py-1 rounded-md hover:bg-primary-50 dark:hover:bg-primary-900/20 transition-colors"
                                                    >
                                                        Add User
                                                    </button>
                                                    <ChevronRight className="w-5 h-5 text-gray-400 group-hover:text-gray-600 dark:group-hover:text-gray-300 transition-colors" />
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
