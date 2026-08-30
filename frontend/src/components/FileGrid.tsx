export function FileGrid() {
    return (
        <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 gap-4">
            {/* File items */}
            <div className="aspect-square bg-gray-100 rounded-lg flex items-center justify-center">
                File
            </div>
        </div>
    );
}
