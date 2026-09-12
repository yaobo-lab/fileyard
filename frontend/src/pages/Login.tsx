import * as React from 'react';
import { useState, useEffect, useCallback } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useAuth } from '@/context/AuthContext';
import { useI18n, useTranslations } from '@/context/I18nContext';
import { useModalDialog } from '@/context/ModalDialogContext';
import {
  Lock,
  Mail,
  AlertCircle,
  ShieldAlert,
  LogIn,
  KeyRound,
  ArrowLeft,
  Loader2,
  Globe,
} from 'lucide-react';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Separator } from '@/components/ui/separator';
import { Checkbox } from '@/components/ui/checkbox';

const API_URL = import.meta.env.VITE_API_URL || '';

interface SsoProvider {
  id: string;
  name: string;
  slug: string;
  provider_type: string;
  protocol: 'oidc' | 'saml';
}

// ─── 第三方登录品牌图标 (企业微信, 飞书, 钉钉) ─────────────────────────────────

function WeComIcon({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path
        d="M8.5 4C4.91 4 2 6.69 2 10c0 1.88.94 3.55 2.41 4.68l-.58 2.37a.5.5 0 0 0 .61.6l2.45-.76c.5.15 1.05.24 1.61.24.4 0 .79-.05 1.17-.13-.11-.47-.17-.95-.17-1.45 0-3.62 3.19-6.55 7.12-6.55.22 0 .44.01.66.03C16.32 6.13 12.72 4 8.5 4z"
        fill="#07C160"
      />
      <path
        d="M16.5 10c-3.31 0-6 2.46-6 5.5s2.69 5.5 6 5.5c.52 0 1.02-.08 1.49-.22l2.05.64a.42.42 0 0 0 .51-.5l-.49-1.99c1.23-.95 2.02-2.35 2.02-3.93 0-3.04-2.69-5.5-6-5.5z"
        fill="#2E75E6"
      />
    </svg>
  );
}

function FeishuIcon({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path
        d="M3.2 14.5l5.1-8.8c.6-1 1.9-1.3 2.9-.7l7.5 4.3c1 .6 1.3 1.9.7 2.9l-5.1 8.8c-.6 1-1.9 1.3-2.9.7L3.9 17.4c-1-.6-1.3-1.9-.7-2.9z"
        fill="#3370FF"
      />
      <path
        d="M8.8 6.2l8.2 4.7c.8.5 1.1 1.5.6 2.3L13.8 20c.5-.8.5-1.9 0-2.8l-4.5-7.7c-.5-.9-.5-2.2-.5-3.3z"
        fill="#00D6B9"
      />
      <path
        d="M12.5 12.8l-5 8.6c-.3.5-.8.8-1.4.8l8.3-4.8 1.9-3.3-3.8-1.3z"
        fill="#0052D9"
      />
    </svg>
  );
}

function DingTalkIcon({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
      <path d="M19.98 9.38c-.37-.84-1.02-1.5-1.84-1.87l-6.85-3.11a3.54 3.54 0 0 0-2.96.06L4.7 6.36C3.65 6.89 3 7.96 3 9.14v5.72c0 1.18.65 2.25 1.7 2.78l3.63 1.9a3.54 3.54 0 0 0 2.96-.06l2.16-1.12c.32-.17.51-.5.51-.86 0-.55-.45-1-1-1-.18 0-.35.05-.5.13l-1.9 1c-.53.27-1.16.27-1.69 0L5.59 16c-.53-.27-.86-.81-.86-1.41V9.41c0-.6.33-1.14.86-1.41l3.28-1.74c.53-.27 1.16-.27 1.69 0l6.23 2.83c.42.19.75.52.94.94.19.42.2.89.04 1.32-.23.63-.77 1.08-1.43 1.21l-3.97.79c-.54.11-.9.63-.79 1.17.11.54.63.9 1.17.79l3.97-.79c1.32-.26 2.39-1.16 2.85-2.42.45-1.26.25-2.66-.58-3.55z" />
      <path d="M15.42 14.15l-3.32.66a.8.8 0 0 0-.63.94.8.8 0 0 0 .94.63l3.32-.66a.8.8 0 0 0 .63-.94.8.8 0 0 0-.94-.63z" />
    </svg>
  );
}

// ─── 登录页面组件 ─────────────────────────────────────────────────────────────

export function Login() {
  const { locale, setLocale } = useI18n();
  const t = useTranslations('Login');
  const { alert: modalAlert } = useModalDialog();

  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [code, setCode] = useState('');
  const [show2FA, setShow2FA] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [isSuspended, setIsSuspended] = useState(false);
  const [rememberMe, setRememberMe] = useState(false);
  const [ssoProviders, setSsoProviders] = useState<SsoProvider[]>([]);
  const [ssoOnly, setSsoOnly] = useState(false);
  const [, setSsoLoading] = useState(false);
  const { login } = useAuth();
  const [searchParams] = useSearchParams();

  // Handle error from SSO redirect
  useEffect(() => {
    const errorParam = searchParams.get('error');
    if (errorParam === 'no_account') {
      setError(t('noSsoAccount'));
    } else if (errorParam === 'no_email') {
      setError(t('noSsoEmail'));
    } else if (errorParam === 'suspended') {
      setIsSuspended(true);
      setError(t('contactAdminSuspended'));
    } else if (errorParam === 'oidc_error' || errorParam === 'saml_error') {
      setError(searchParams.get('message') || t('ssoFailed'));
    }
  }, [searchParams, t]);

  // Discover SSO providers when email changes
  const discoverProviders = useCallback(async (emailValue: string) => {
    if (!emailValue || !emailValue.includes('@')) {
      setSsoProviders([]);
      setSsoOnly(false);
      return;
    }

    setSsoLoading(true);
    try {
      const response = await fetch(
        `${API_URL}/api/auth/oidc/providers?email=${encodeURIComponent(emailValue)}`
      );
      if (response.ok) {
        const data = await response.json();
        setSsoProviders(data.providers || []);
        setSsoOnly(data.sso_only || false);
      }
    } catch {
      // Silently ignore — SSO discovery is optional
    } finally {
      setSsoLoading(false);
    }
  }, []);

  const handleEmailBlur = () => {
    discoverProviders(email);
  };

  const handleSsoLogin = (provider: SsoProvider) => {
    const protocol = provider.protocol || 'oidc';
    window.location.href = `${API_URL}/api/auth/${protocol}/authorize/${provider.id}`;
  };

  // 第三方登录点击处理（飞书、钉钉、企业微信）
  const handleThirdPartyLogin = (platform: 'wecom' | 'feishu' | 'dingtalk') => {
    modalAlert({
      title: t('featureInDevelopment'),
      description: t('featureInDevelopmentDesc'),
      variant: 'info',
    });
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError('');
    setIsSuspended(false);

    try {
      const result = await login(email, password, code, rememberMe);

      // Check for suspended account or company
      if (
        result &&
        (result.error === 'account_suspended' || result.error === 'company_suspended')
      ) {
        setIsSuspended(true);
        setError(result.message || t('contactAdminSuspended'));
        setLoading(false);
        return;
      }

      // SSO required — show SSO buttons
      if (result && result.error === 'sso_required') {
        if (result.providers && result.providers.length > 0) {
          setSsoProviders(result.providers);
          setSsoOnly(true);
        }
        setError(result.message || t('ssoRequired'));
        setLoading(false);
        return;
      }

      if (result && result.require_2fa) {
        setShow2FA(true);
        setLoading(false);
        return;
      }
    } catch (err: any) {
      console.error('Login error:', err);
      setError(t('invalidCredentials'));
      setLoading(false);
    }
  };

  return (
    <div className='min-h-svh w-full flex items-center justify-center p-4 bg-background relative'>
      {/* 语言切换器 */}
      <div className='absolute top-4 right-4 z-20'>
        <button
          type='button'
          onClick={() => setLocale(locale === 'zh' ? 'en' : 'zh')}
          className='flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-muted-foreground hover:text-foreground bg-card/80 hover:bg-card border border-border rounded-lg shadow-xs backdrop-blur-xs transition-colors cursor-pointer'
        >
          <Globe className='size-3.5' />
          <span>{locale === 'zh' ? 'English' : '简体中文'}</span>
        </button>
      </div>

      <div className='w-full max-w-md space-y-6'>
        {/* Brand Header (已移除 SVG Logo) */}
        <div className='flex flex-col items-center text-center space-y-2'>
          <h1 className='text-2xl font-bold tracking-tight'>{t('welcomeTitle')}</h1>
          <p className='text-sm text-muted-foreground'>
            {t('welcomeSubtitle')}
          </p>
        </div>

        {/* Main Card */}
        <Card className='shadow-lg border-border'>
          <CardHeader className='pb-4'>
            <CardTitle className='text-xl'>
              {show2FA ? t('twoFactorTitle') : t('signInTitle')}
            </CardTitle>
            <CardDescription>
              {show2FA ? t('twoFactorSubtitle') : t('signInSubtitle')}
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
                  {isSuspended ? t('accountSuspended') : t('authError')}
                </AlertTitle>
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}

            <form onSubmit={handleSubmit} className='space-y-4'>
              {!show2FA ? (
                <>
                  {/* Email Field */}
                  <div className='space-y-1.5'>
                    <Label htmlFor='email'>{t('emailLabel')}</Label>
                    <div className='relative'>
                      <Mail className='absolute left-3 top-2.5 size-4 text-muted-foreground' />
                      <Input
                        id='email'
                        type='email'
                        required
                        autoComplete='email'
                        placeholder={t('emailPlaceholder')}
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
                              {t('orSso')}
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
                            <span>{t('continueWith', { name: provider.name })}</span>
                          </Button>
                        ))}
                      </div>

                      {ssoOnly && (
                        <p className='text-xs text-center text-muted-foreground pt-1'>
                          {t('ssoEnforced')}
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
                              {t('orPassword')}
                            </span>
                          </div>
                        </div>
                      )}

                      <div className='space-y-1.5'>
                        <div className='flex items-center justify-between'>
                          <Label htmlFor='password'>{t('passwordLabel')}</Label>
                          <button
                            type='button'
                            onClick={() =>
                              modalAlert({
                                title: t('forgotPassword'),
                                description: t('forgotPasswordMsg'),
                                variant: 'info',
                              })
                            }
                            className='text-xs text-muted-foreground hover:text-foreground underline underline-offset-4 cursor-pointer'
                          >
                            {t('forgotPassword')}
                          </button>
                        </div>
                        <div className='relative'>
                          <Lock className='absolute left-3 top-2.5 size-4 text-muted-foreground' />
                          <Input
                            id='password'
                            type='password'
                            required={!ssoOnly}
                            autoComplete='current-password'
                            placeholder={t('passwordPlaceholder')}
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
                          {t('rememberMe')}
                        </Label>
                      </div>
                    </>
                  )}
                </>
              ) : (
                /* 2FA Mode */
                <div className='space-y-3'>
                  <div className='space-y-1.5'>
                    <Label htmlFor='code'>{t('authCodeLabel')}</Label>
                    <div className='relative'>
                      <KeyRound className='absolute left-3 top-2.5 size-4 text-muted-foreground' />
                      <Input
                        id='code'
                        type='text'
                        required
                        autoFocus
                        placeholder={t('authCodePlaceholder')}
                        value={code}
                        onChange={(e) => setCode(e.target.value)}
                        className='pl-9 text-center tracking-widest text-lg font-mono'
                        maxLength={8}
                      />
                    </div>
                    <p className='text-xs text-muted-foreground'>
                      {t('authCodeHint')}
                    </p>
                  </div>

                  <Button
                    type='button'
                    variant='ghost'
                    size='sm'
                    className='w-full text-xs text-muted-foreground'
                    onClick={() => {
                      setShow2FA(false);
                      setCode('');
                    }}
                  >
                    <ArrowLeft className='size-3 mr-1' />
                    {t('backToPassword')}
                  </Button>
                </div>
              )}

              {/* Submit Button */}
              {(!ssoOnly || show2FA) && (
                <Button
                  type='submit'
                  className='w-full mt-2 font-medium cursor-pointer'
                  disabled={loading}
                >
                  {loading && <Loader2 className='size-4 animate-spin mr-2' />}
                  {show2FA ? t('verifyCodeBtn') : (loading ? t('signingIn') : t('signInBtn'))}
                </Button>
              )}
            </form>

            {/* 第三方企业登录 (企业微信、飞书、钉钉) */}
            {!show2FA && (
              <div className='mt-6 pt-2'>
                <div className='relative mb-4'>
                  <div className='absolute inset-0 flex items-center'>
                    <Separator />
                  </div>
                  <div className='relative flex justify-center text-xs uppercase'>
                    <span className='bg-card px-2 text-muted-foreground font-medium'>
                      {t('orThirdParty')}
                    </span>
                  </div>
                </div>

                <div className='grid grid-cols-3 gap-2.5'>
                  {/* 企业微信 */}
                  <Button
                    type='button'
                    variant='outline'
                    className='h-10 px-2 flex items-center justify-center gap-1.5 hover:bg-muted/80 hover:border-emerald-500/50 hover:text-emerald-600 dark:hover:text-emerald-400 transition-all text-xs font-medium cursor-pointer'
                    onClick={() => handleThirdPartyLogin('wecom')}
                    title={t('wecom')}
                  >
                    <WeComIcon className='size-4 shrink-0' />
                    <span className='truncate'>{t('wecom')}</span>
                  </Button>

                  {/* 飞书 */}
                  <Button
                    type='button'
                    variant='outline'
                    className='h-10 px-2 flex items-center justify-center gap-1.5 hover:bg-muted/80 hover:border-blue-500/50 hover:text-blue-600 dark:hover:text-blue-400 transition-all text-xs font-medium cursor-pointer'
                    onClick={() => handleThirdPartyLogin('feishu')}
                    title={t('feishu')}
                  >
                    <FeishuIcon className='size-4 shrink-0' />
                    <span className='truncate'>{t('feishu')}</span>
                  </Button>

                  {/* 钉钉 */}
                  <Button
                    type='button'
                    variant='outline'
                    className='h-10 px-2 flex items-center justify-center gap-1.5 hover:bg-muted/80 hover:border-sky-500/50 hover:text-sky-600 dark:hover:text-sky-400 transition-all text-xs font-medium cursor-pointer'
                    onClick={() => handleThirdPartyLogin('dingtalk')}
                    title={t('dingtalk')}
                  >
                    <DingTalkIcon className='size-4 shrink-0 text-[#0089FF]' />
                    <span className='truncate'>{t('dingtalk')}</span>
                  </Button>
                </div>
              </div>
            )}
          </CardContent>

          {/* 注：原 CardFooter (包含隐私政策、服务条款链接及版权年份) 已按要求完全移除 */}
        </Card>
      </div>
    </div>
  );
}
