import * as React from 'react'
import { useNavigate } from 'react-router-dom'
import { Search, Building2, User, Folder, FileText, Loader2 } from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { useAuthFetch } from '@/context/AuthContext'

interface SearchResult {
  id: string
  name: string
  description?: string
  result_type: 'company' | 'user' | 'file' | 'folder' | 'group'
  link: string
}

interface SearchResults {
  companies?: SearchResult[]
  users?: SearchResult[]
  files?: SearchResult[]
  groups?: SearchResult[]
  total?: number
}

interface SearchDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  placeholder?: string
}

export function SearchDialog({
  open,
  onOpenChange,
  placeholder = 'Search companies, users, or files...',
}: SearchDialogProps) {
  const [query, setQuery] = React.useState('')
  const [results, setResults] = React.useState<SearchResults | null>(null)
  const [isLoading, setIsLoading] = React.useState(false)
  const searchTimeoutRef = React.useRef<ReturnType<typeof setTimeout> | null>(null)
  const inputRef = React.useRef<HTMLInputElement>(null)

  const authFetch = useAuthFetch()
  const navigate = useNavigate()

  // Focus input on open
  React.useEffect(() => {
    if (open) {
      setTimeout(() => inputRef.current?.focus(), 50)
    } else {
      setQuery('')
      setResults(null)
    }
  }, [open])

  // Debounced search
  const handleSearch = React.useCallback(
    async (searchVal: string) => {
      if (searchVal.trim().length < 2) {
        setResults(null)
        return
      }

      setIsLoading(true)
      try {
        const response = await authFetch(
          `/api/search?q=${encodeURIComponent(searchVal)}&limit=5`
        )
        if (response.ok) {
          const data = await response.json()
          setResults(data)
        }
      } catch (err) {
        console.error('Search failed:', err)
      } finally {
        setIsLoading(false)
      }
    },
    [authFetch]
  )

  const onChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value
    setQuery(val)

    if (searchTimeoutRef.current) {
      clearTimeout(searchTimeoutRef.current)
    }

    searchTimeoutRef.current = setTimeout(() => {
      handleSearch(val)
    }, 250)
  }

  const handleSelect = (link: string) => {
    onOpenChange(false)
    navigate(link)
  }

  const getResultIcon = (type: string) => {
    switch (type) {
      case 'company':
        return <Building2 className='size-4 text-blue-500' />
      case 'user':
        return <User className='size-4 text-emerald-500' />
      case 'file':
        return <Folder className='size-4 text-amber-500' />
      default:
        return <FileText className='size-4 text-muted-foreground' />
    }
  }

  const hasResults =
    results &&
    ((results.companies && results.companies.length > 0) ||
      (results.users && results.users.length > 0) ||
      (results.files && results.files.length > 0) ||
      (results.groups && results.groups.length > 0))

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='p-0 overflow-hidden sm:max-w-lg shadow-2xl'>
        <DialogHeader className='sr-only'>
          <DialogTitle>Search</DialogTitle>
        </DialogHeader>

        <div className='flex items-center border-b px-3 bg-card'>
          <Search className='size-4 shrink-0 text-muted-foreground me-2' />
          <input
            ref={inputRef}
            type='text'
            value={query}
            onChange={onChange}
            placeholder={placeholder}
            className='flex h-11 w-full rounded-md bg-transparent py-3 text-sm outline-none placeholder:text-muted-foreground disabled:cursor-not-allowed disabled:opacity-50'
          />
          {isLoading && (
            <Loader2 className='size-4 animate-spin text-muted-foreground ms-2' />
          )}
        </div>

        <div className='max-h-80 overflow-y-auto p-2'>
          {query.trim().length >= 2 && !isLoading && !hasResults && (
            <div className='p-6 text-center text-sm text-muted-foreground'>
              No results found for "{query}".
            </div>
          )}

          {query.trim().length < 2 && (
            <div className='p-6 text-center text-xs text-muted-foreground'>
              Type at least 2 characters to search...
            </div>
          )}

          {results?.companies && results.companies.length > 0 && (
            <div className='mb-2'>
              <div className='px-2 py-1 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground'>
                Companies
              </div>
              {results.companies.map((item) => (
                <div
                  key={item.id}
                  onClick={() => handleSelect(item.link)}
                  className='flex items-center gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-accent hover:text-accent-foreground cursor-pointer transition-colors'
                >
                  {getResultIcon(item.result_type)}
                  <span className='font-medium'>{item.name}</span>
                </div>
              ))}
            </div>
          )}

          {results?.users && results.users.length > 0 && (
            <div className='mb-2'>
              <div className='px-2 py-1 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground'>
                Users
              </div>
              {results.users.map((item) => (
                <div
                  key={item.id}
                  onClick={() => handleSelect(item.link)}
                  className='flex items-center gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-accent hover:text-accent-foreground cursor-pointer transition-colors'
                >
                  {getResultIcon(item.result_type)}
                  <div className='flex flex-col'>
                    <span className='font-medium'>{item.name}</span>
                    {item.description && (
                      <span className='text-xs text-muted-foreground'>{item.description}</span>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}

          {results?.files && results.files.length > 0 && (
            <div className='mb-2'>
              <div className='px-2 py-1 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground'>
                Files & Folders
              </div>
              {results.files.map((item) => (
                <div
                  key={item.id}
                  onClick={() => handleSelect(item.link)}
                  className='flex items-center gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-accent hover:text-accent-foreground cursor-pointer transition-colors'
                >
                  {getResultIcon(item.result_type)}
                  <span className='font-medium'>{item.name}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  )
}
