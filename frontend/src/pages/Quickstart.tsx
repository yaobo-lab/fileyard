import { ArrowLeft, Upload, FolderPlus, Share2, Download, Link2, Search, History, Bell, User } from 'lucide-react';
import { Link } from 'react-router-dom';

export default function Quickstart() {
    const guides = [
        {
            icon: Upload,
            title: 'Uploading Files',
            description: 'Add files to your workspace',
            steps: [
                'Navigate to the Files section from the sidebar',
                'Click the "Upload" button or drag and drop files directly into the browser',
                'Wait for the upload to complete - you\'ll see a progress indicator',
                'Your files are now securely stored and accessible'
            ]
        },
        {
            icon: FolderPlus,
            title: 'Creating Folders',
            description: 'Organize your files into folders',
            steps: [
                'Go to the Files section',
                'Click the "New Folder" button',
                'Enter a name for your folder',
                'Click "Create" to add the folder to your current location'
            ]
        },
        {
            icon: Share2,
            title: 'Sharing Files',
            description: 'Share files with team members',
            steps: [
                'Find the file you want to share',
                'Click the three-dot menu (⋯) on the file',
                'Select "Share" from the dropdown',
                'Choose who to share with and set permissions',
                'Copy the share link or send directly to users'
            ]
        },
        {
            icon: Download,
            title: 'Downloading Files',
            description: 'Download files to your device',
            steps: [
                'Locate the file you need',
                'Click on the file to preview it, or use the three-dot menu',
                'Select "Download" to save the file to your device',
                'For multiple files, select them and use bulk download'
            ]
        },
        {
            icon: Link2,
            title: 'File Requests',
            description: 'Collect files from others securely',
            steps: [
                'Go to "Requests" in the sidebar',
                'Click "Create Request" button',
                'Set a name, destination folder, and expiration',
                'Share the generated link with people outside your organization',
                'They can upload files directly without needing an account'
            ]
        },
        {
            icon: Search,
            title: 'Searching',
            description: 'Find files, users, and companies quickly',
            steps: [
                'Use the search bar at the top of the page',
                'Type your search query - it searches files, users, and companies',
                'Results appear as you type',
                'Click on a result to navigate directly to it'
            ]
        },
        {
            icon: History,
            title: 'File History',
            description: 'View and restore previous versions',
            steps: [
                'Click on a file to open the preview',
                'Select "Properties" or "Activity" from the file menu',
                'View the complete history of changes',
                'Download or restore previous versions if needed'
            ]
        },
        {
            icon: Bell,
            title: 'Notifications',
            description: 'Stay updated on important events',
            steps: [
                'Click the bell icon in the top navigation',
                'View recent notifications about shares, uploads, and mentions',
                'Click a notification to go directly to the related item',
                'Customize notification preferences in Settings'
            ]
        },
        {
            icon: User,
            title: 'Profile Settings',
            description: 'Manage your account',
            steps: [
                'Click your name/avatar in the sidebar',
                'Update your profile information',
                'Enable two-factor authentication for added security',
                'Manage your notification and display preferences'
            ]
        }
    ];

    return (
        <div className="max-w-5xl mx-auto">
            <Link 
                to="/" 
                className="inline-flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white mb-6"
            >
                <ArrowLeft className="w-4 h-4" />
                Back to Dashboard
            </Link>

            <div className="mb-8">
                <h1 className="text-3xl font-bold text-gray-900 dark:text-white">Quickstart Guide</h1>
                <p className="mt-2 text-lg text-gray-600 dark:text-gray-400">
                    Get up and running with ClovaLink in minutes. Learn the basics to start managing your files.
                </p>
            </div>

            <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
                {guides.map((guide, index) => (
                    <div 
                        key={index}
                        className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700 p-5 hover:shadow-md transition-shadow"
                    >
                        <div className="flex items-center gap-3 mb-4">
                            <div className="p-2 bg-primary-100 dark:bg-primary-900/30 rounded-lg">
                                <guide.icon className="w-5 h-5 text-primary-600 dark:text-primary-400" />
                            </div>
                            <div>
                                <h3 className="font-semibold text-gray-900 dark:text-white">{guide.title}</h3>
                                <p className="text-xs text-gray-500 dark:text-gray-400">{guide.description}</p>
                            </div>
                        </div>
                        <ol className="space-y-2">
                            {guide.steps.map((step, stepIndex) => (
                                <li key={stepIndex} className="flex gap-3 text-sm text-gray-600 dark:text-gray-400">
                                    <span className="flex-shrink-0 w-5 h-5 rounded-full bg-gray-100 dark:bg-gray-700 text-gray-500 dark:text-gray-400 text-xs flex items-center justify-center font-medium">
                                        {stepIndex + 1}
                                    </span>
                                    <span>{step}</span>
                                </li>
                            ))}
                        </ol>
                    </div>
                ))}
            </div>

            <div className="mt-8 bg-primary-50 dark:bg-primary-900/20 border border-primary-200 dark:border-primary-800 rounded-lg p-6">
                <h2 className="text-lg font-semibold text-primary-900 dark:text-primary-100 mb-2">Need More Help?</h2>
                <p className="text-primary-700 dark:text-primary-300 mb-4">
                    For detailed documentation on compliance modes, retention policies, and advanced features:
                </p>
                <div className="flex flex-wrap gap-3">
                    <Link 
                        to="/help"
                        className="inline-flex items-center px-4 py-2 bg-primary-600 text-white rounded-md hover:bg-primary-700 transition-colors text-sm font-medium"
                    >
                        View Help Documentation
                    </Link>
                    <a 
                        href="https://clovalink.org/docs"
                        target="_blank"
                        rel="noopener noreferrer"
                        className="inline-flex items-center px-4 py-2 bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 border border-gray-300 dark:border-gray-600 rounded-md hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors text-sm font-medium"
                    >
                        Full Documentation
                    </a>
                </div>
            </div>
        </div>
    );
}
