export function TenantSwitcher() {
    return (
        <div className="min-h-screen flex items-center justify-center bg-gray-50">
            <div className="max-w-md w-full bg-white p-8 rounded-lg shadow-sm border border-gray-200">
                <h2 className="text-2xl font-bold text-center mb-6">Select Company</h2>
                <div className="space-y-4">
                    <button className="w-full p-4 text-left border rounded-lg hover:bg-gray-50">
                        <div className="font-medium">Acme Corp</div>
                        <div className="text-sm text-gray-500">Admin</div>
                    </button>
                    <button className="w-full p-4 text-left border rounded-lg hover:bg-gray-50">
                        <div className="font-medium">Globex Inc</div>
                        <div className="text-sm text-gray-500">Manager</div>
                    </button>
                </div>
            </div>
        </div>
    );
}
