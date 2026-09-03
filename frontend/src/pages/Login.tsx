import * as React from 'react'
import { useState, useEffect, useCallback } from 'react'
import { useSearchParams, Link } from 'react-router-dom'
import { useAuth } from '@/context/AuthContext'
import {
  Lock,
  Mail,
  AlertCircle,
  ShieldAlert,
  LogIn,
  KeyRound,
  ArrowLeft,
  Loader2,
} from 'lucide-react'
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { Separator } from '@/components/ui/separator'
import { Checkbox } from '@/components/ui/checkbox'
import { Logo } from '@/components/Logo'

const API_URL = import.meta.env.VITE_API_URL || ''

interface SsoProvider {
  id: string
  name: string
  slug: string
  provider_type: string
  protocol: 'oidc' | 'saml'
}

export function Login() {
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [code, setCode] = useState('')
  const [show2FA, setShow2FA] = useState(false)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [isSuspended, setIsSuspended] = useState(false)
  const [rememberMe, setRememberMe] = useState(false)
  const [ssoProviders, setSsoProviders] = useState<SsoProvider[]>([])
  const [ssoOnly, setSsoOnly] = useState(false)
  const [ssoLoading, setSsoLoading] = useState(false)
  const { login } = useAuth()
  const [searchParams] = useSearchParams()

  // Handle error from SSO redirect
  useEffect(() => {
    const errorParam = searchParams.get('error')
    if (errorParam === 'no_account') {
      setError('No account found for this SSO identity. Contact your administrator.')
    } else if (errorParam === 'no_email') {
      setError('SSO provider did not return an email address.')
    } else if (errorParam === 'suspended') {
      setIsSuspended(true)
      setError('Your account is suspended. Contact your administrator.')
    } else if (errorParam === 'oidc_error' || errorParam === 'saml_error') {
      setError(searchParams.get('message') || 'SSO authentication failed.')
    }
  }, [searchParams])

  // Discover SSO providers when email changes
  const discoverProviders = useCallback(async (emailValue: string) => {
    if (!emailValue || !emailValue.includes('@')) {
      setSsoProviders([])
      setSsoOnly(false)
      return
    }

    setSsoLoading(true)
    try {
      const response = await fetch(
        `${API_URL}/api/auth/oidc/providers?email=${encodeURIComponent(emailValue)}`
      )
      if (response.ok) {
        const data = await response.json()
        setSsoProviders(data.providers || [])
        setSsoOnly(data.sso_only || false)
      }
    } catch {
      // Silently ignore — SSO discovery is optional
    } finally {
      setSsoLoading(false)
    }
  }, [])

  const handleEmailBlur = () => {
    discoverProviders(email)
  }

  const handleSsoLogin = (provider: SsoProvider) => {
    const protocol = provider.protocol || 'oidc'
    window.location.href = `${API_URL}/api/auth/${protocol}/authorize/${provider.id}`
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setLoading(true)
    setError('')
    setIsSuspended(false)

    try {
      const result = await login(email, password, code, rememberMe)

      // Check for suspended account or company
      if (
        result &&
        (result.error === 'account_suspended' || result.error === 'company_suspended')
      ) {
        setIsSuspended(true)
        setError(result.message || 'Access denied. Please contact your administrator.')
        setLoading(false)
        return
      }

      // SSO required — show SSO buttons
      if (result && result.error === 'sso_required') {
        if (result.providers && result.providers.length > 0) {
          setSsoProviders(result.providers)
          setSsoOnly(true)
        }
        setError(
          result.message ||
            'This account uses SSO. Please sign in with your identity provider.'
        )
        setLoading(false)
        return
      }

      if (result && result.require_2fa) {
        setShow2FA(true)
        setLoading(false)
        return
      }
    } catch (err: any) {
      console.error('Login error:', err)
      setError('Invalid email, password, or 2FA code. Please try again.')
      setLoading(false)
    }
  }

  return (
    <div className='min-h-svh w-full flex items-center justify-center p-4 bg-background'>
      <div className='w-full max-w-md space-y-6'>
        {/* Brand Header */}
        <div className='flex flex-col items-center text-center space-y-2'>
          <div className='flex items-center justify-center p-2'>
            <Logo className='h-12 w-auto' />
          </div>
          <h1 className='text-2xl font-bold tracking-tight'>Welcome to Fileyard</h1>
          <p className='text-sm text-muted-foreground'>
            Secure file sharing, management and collaboration platform
          </p>
        </div>

        {/* Main Card */}
        <Card className='shadow-lg border-border'>
          <CardHeader className='pb-4'>
            <CardTitle className='text-xl'>
              {show2FA ? 'Two-Factor Authentication' : 'Sign in'}
            </CardTitle>
            <CardDescription>
              {show2FA
                ? 'Enter the 6-digit authentication code from your authenticator app.'
                : 'Enter your credentials to access your workspace.'}
            </CardDescription>
          </CardHeader>

          <CardContent>
            {/* Error Alert */}
            {error && (
              <Alert
                variant='destructive'
                className='mb-4 text-xs'
              >
                {isSuspended ? (
                  <ShieldAlert className='size-4' />
                ) : (
                  <AlertCircle className='size-4' />
                )}
                <AlertTitle className='font-semibold'>
                  {isSuspended ? 'Account Suspended' : 'Authentication Error'}
                </AlertTitle>
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}

            <form onSubmit={handleSubmit} className='space-y-4'>
              {!show2FA ? (
                <>
                  {/* Email Field */}
                  <div className='space-y-1.5'>
                    <Label htmlFor='email'>Email address</Label>
                    <div className='relative'>
                      <Mail className='absolute left-3 top-2.5 size-4 text-muted-foreground' />
                      <Input
                        id='email'
                        type='email'
                        required
                        autoComplete='email'
                        placeholder='name@example.com'
                        value={email}
                        onChange={(e) => setEmail(e.target.value)}
                        onBlur={handleEmailBlur}
                        className='pl-9'
                      />
                    </div>
                  </div>

                  {/* SSO Discovery Buttons */}
                  {ssoProviders.length > 0 && (
                    <div className='space-y-2 pt-2'>
                      {!ssoOnly && (
                        <div className='relative my-3'>
                          <div className='absolute inset-0 flex items-center'>
                            <Separator />
                          </div>
                          <div className='relative flex justify-center text-xs uppercase'>
                            <span className='bg-card px-2 text-muted-foreground font-medium'>
                              Or sign in with SSO
                            </span>
                          </div>
                        </div>
                      )}

                      <div className='space-y-2'>
                        {ssoProviders.map((provider) => (
                          <Button
                            key={provider.id}
                            type='button'
                            variant='outline'
                            className='w-full justify-center gap-2'
                            onClick={() => handleSsoLogin(provider)}
                          >
                            <LogIn className='size-4' />
                            <span>Continue with {provider.name}</span>
                          </Button>
                        ))}
                      </div>

                      {ssoOnly && (
                        <p className='text-xs text-center text-muted-foreground pt-1'>
                          Your organization enforces SSO authentication.
                        </p>
                      )}
                    </div>
                  )}

                  {/* Password Field */}
                  {!ssoOnly && (
                    <>
                      {ssoProviders.length > 0 && (
                        <div className='relative my-3'>
                          <div className='absolute inset-0 flex items-center'>
                            <Separator />
                          </div>
                          <div className='relative flex justify-center text-xs uppercase'>
                            <span className='bg-card px-2 text-muted-foreground font-medium'>
                              Or with password
                            </span>
                          </div>
                        </div>
                      )}

                      <div className='space-y-1.5'>
                        <div className='flex items-center justify-between'>
                          <Label htmlFor='password'>Password</Label>
                          <button
                            type='button'
                            onClick={() =>
                              setError('Please contact your administrator to reset your password.')
                            }
                            className='text-xs text-muted-foreground hover:text-foreground underline underline-offset-4 cursor-pointer'
                          >
                            Forgot password?
                          </button>
                        </div>
                        <div className='relative'>
                          <Lock className='absolute left-3 top-2.5 size-4 text-muted-foreground' />
                          <Input
                            id='password'
                            type='password'
                            required={!ssoOnly}
                            autoComplete='current-password'
                            placeholder='••••••••'
                            value={password}
                            onChange={(e) => setPassword(e.target.value)}
                            className='pl-9'
                          />
                        </div>
                      </div>

                      {/* Remember me */}
                      <div className='flex items-center space-x-2 pt-1'>
                        <Checkbox
                          id='remember-me'
                          checked={rememberMe}
                          onCheckedChange={(checked) => setRememberMe(!!checked)}
                        />
                        <Label
                          htmlFor='remember-me'
                          className='text-xs font-normal text-muted-foreground cursor-pointer'
                        >
                          Remember me on this device
                        </Label>
                      </div>
                    </>
                  )}
                </>
              ) : (
                /* 2FA Mode */
                <div className='space-y-3'>
                  <div className='space-y-1.5'>
                    <Label htmlFor='code'>Authentication Code</Label>
                    <div className='relative'>
                      <KeyRound className='absolute left-3 top-2.5 size-4 text-muted-foreground' />
                      <Input
                        id='code'
                        type='text'
                        required
                        autoFocus
                        placeholder='123456'
                        value={code}
                        onChange={(e) => setCode(e.target.value)}
                        className='pl-9 text-center tracking-widest text-lg font-mono'
                        maxLength={8}
                      />
                    </div>
                    <p className='text-xs text-muted-foreground'>
                      Open your authenticator app (Google Authenticator, Microsoft Authenticator) to retrieve the 6-digit code.
                    </p>
                  </div>

                  <Button
                    type='button'
                    variant='ghost'
                    size='sm'
                    className='w-full text-xs text-muted-foreground'
                    onClick={() => {
                      setShow2FA(false)
                      setCode('')
                    }}
                  >
                    <ArrowLeft className='size-3 mr-1' />
                    Back to password sign in
                  </Button>
                </div>
              )}

              {/* Submit Button */}
              {(!ssoOnly || show2FA) && (
                <Button
                  type='submit'
                  className='w-full mt-2 font-medium'
                  disabled={loading}
                >
                  {loading && <Loader2 className='size-4 animate-spin mr-2' />}
                  {show2FA ? 'Verify Code' : 'Sign in'}
                </Button>
              )}
            </form>
          </CardContent>

          <CardFooter className='flex flex-col items-center justify-center border-t py-4 text-xs text-muted-foreground space-y-2'>
            <div className='flex items-center gap-4'>
              <Link to='/privacy' className='hover:text-foreground underline underline-offset-4'>
                Privacy Policy
              </Link>
              <span>•</span>
              <Link to='/terms' className='hover:text-foreground underline underline-offset-4'>
                Terms of Service
              </Link>
            </div>
            <span>
              &copy; {new Date().getFullYear()} Fileyard. All rights reserved.
            </span>
          </CardFooter>
        </Card>
      </div>
    </div>
  )
}
