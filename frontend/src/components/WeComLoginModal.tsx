import React, { useEffect, useState, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { X, ExternalLink, RefreshCw, AlertCircle, Sparkles, CheckCircle2, ShieldCheck, Loader2 } from 'lucide-react';
import { useTranslations } from '@/context/I18nContext';
import { useAuth } from '@/context/AuthContext';
import { Button } from '@/components/ui/button';

const API_URL = import.meta.env.VITE_API_URL || '';

interface WeComConfig {
  enabled: boolean;
  configured: boolean;
  corp_id: string;
  agent_id: string;
  redirect_uri: string;
}

interface WeComLoginModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export function WeComIcon({ className }: { className?: string }) {
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

export function WeComLoginModal({ isOpen, onClose }: WeComLoginModalProps) {
  const t = useTranslations('Login');
  const navigate = useNavigate();
  const { refreshUser } = useAuth();

  const [loading, setLoading] = useState(true);
  const [config, setConfig] = useState<WeComConfig | null>(null);
  const [demoLoading, setDemoLoading] = useState(false);
  const [demoSuccess, setDemoSuccess] = useState(false);
  const [scriptLoaded, setScriptLoaded] = useState(false);
  const qrContainerRef = useRef<HTMLDivElement>(null);

  // 加载配置
  useEffect(() => {
    if (!isOpen) return;

    let isMounted = true;
    setLoading(true);

    fetch(`${API_URL}/api/auth/wecom/config`)
      .then((res) => (res.ok ? res.json() : null))
      .then((data: WeComConfig | null) => {
        if (!isMounted) return;
        setConfig(data);
        setLoading(false);
      })
      .catch((err) => {
        console.error('Failed to load WeCom config:', err);
        if (isMounted) setLoading(false);
      });

    return () => {
      isMounted = false;
    };
  }, [isOpen]);

  // 如果已配置，动态载入企业微信官方扫码 JS
  useEffect(() => {
    if (!isOpen || !config?.configured) return;

    // 检查是否已有 SDK
    if ((window as any).WwLogin) {
      setScriptLoaded(true);
      return;
    }

    const script = document.createElement('script');
    script.src = 'https://wwcdn.weixin.qq.com/node/wework/wwopen/js/wwLogin-1.2.7.js';
    script.async = true;
    script.onload = () => {
      setScriptLoaded(true);
    };
    document.body.appendChild(script);

    return () => {
      // 保持 script 缓存
    };
  }, [isOpen, config?.configured]);

  // 初始化企微官方扫码组件
  useEffect(() => {
    if (!isOpen || !config?.configured || !scriptLoaded || !qrContainerRef.current) return;

    try {
      qrContainerRef.current.innerHTML = '';
      if ((window as any).WwLogin) {
        new (window as any).WwLogin({
          id: 'wecom_qr_container',
          appid: config.corp_id,
          agentid: config.agent_id,
          redirect_uri: encodeURIComponent(config.redirect_uri),
          state: Math.random().toString(36).substring(2),
          href: '',
        });
      }
    } catch (err) {
      console.error('Failed to instantiate WwLogin:', err);
    }
  }, [isOpen, config, scriptLoaded]);

  // 快捷键 ESC 关闭
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isOpen) {
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  // 快速模拟演示登录
  const handleDemoLogin = async () => {
    setDemoLoading(true);
    try {
      const res = await fetch(`${API_URL}/api/auth/wecom/demo-login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
      });

      if (!res.ok) {
        throw new Error('Demo login failed');
      }

      const data = await res.json();
      sessionStorage.setItem('auth_token', data.token);
      setDemoSuccess(true);

      setTimeout(async () => {
        await refreshUser(data.token);
        onClose();
        navigate('/', { replace: true });
      }, 600);
    } catch (err) {
      console.error('WeCom demo login error:', err);
    } finally {
      setDemoLoading(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-in fade-in duration-200">
      <div
        className="relative w-full max-w-md overflow-hidden bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 rounded-2xl shadow-2xl transition-all"
        onClick={(e) => e.stopPropagation()}
      >
        {/* 顶部标题栏 */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-zinc-100 dark:border-zinc-800 bg-zinc-50/50 dark:bg-zinc-900/50">
          <div className="flex items-center gap-2.5">
            <div className="p-1.5 rounded-lg bg-emerald-50 dark:bg-emerald-950/40 border border-emerald-200/60 dark:border-emerald-800/40">
              <WeComIcon className="size-5" />
            </div>
            <div>
              <h3 className="text-base font-semibold text-zinc-900 dark:text-zinc-100">
                {t('wecomLoginTitle')}
              </h3>
              <p className="text-xs text-zinc-500 dark:text-zinc-400">
                {t('wecomScanHint')}
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-200 hover:bg-zinc-100 dark:hover:bg-zinc-800 rounded-lg transition-colors cursor-pointer"
            aria-label="Close"
          >
            <X className="size-4" />
          </button>
        </div>

        {/* 内容主体 */}
        <div className="p-6">
          {loading ? (
            <div className="flex flex-col items-center justify-center py-16 gap-3">
              <Loader2 className="size-8 text-emerald-500 animate-spin" />
              <span className="text-xs text-zinc-500 dark:text-zinc-400">
                {t('wecomLoadingQr')}
              </span>
            </div>
          ) : config?.configured ? (
            /* 已配置真实企业微信自建固件：渲染官方内嵌二维码 */
            <div className="flex flex-col items-center">
              <div
                id="wecom_qr_container"
                ref={qrContainerRef}
                className="w-[300px] h-[300px] flex items-center justify-center rounded-xl bg-zinc-50 dark:bg-zinc-800/50 border border-dashed border-zinc-200 dark:border-zinc-700 overflow-hidden"
              >
                {!scriptLoaded && (
                  <div className="flex flex-col items-center gap-2 text-zinc-400">
                    <Loader2 className="size-6 animate-spin text-emerald-500" />
                    <span className="text-xs">{t('wecomLoadingQr')}</span>
                  </div>
                )}
              </div>

              {/* 辅助按钮：前往企业微信授权页 */}
              <div className="mt-4 flex items-center justify-between w-full pt-3 border-t border-zinc-100 dark:border-zinc-800 text-xs">
                <a
                  href={`${API_URL}/api/auth/wecom/authorize`}
                  target="_blank"
                  rel="noreferrer"
                  className="inline-flex items-center gap-1 text-emerald-600 dark:text-emerald-400 hover:underline font-medium"
                >
                  <ExternalLink className="size-3.5" />
                  {t('wecomOrRedirect')}
                </a>
                <button
                  type="button"
                  onClick={() => {
                    if (qrContainerRef.current) qrContainerRef.current.innerHTML = '';
                    setScriptLoaded(false);
                    setTimeout(() => setScriptLoaded(true), 100);
                  }}
                  className="inline-flex items-center gap-1 text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300"
                >
                  <RefreshCw className="size-3.5" />
                  {t('wecomRefreshQr')}
                </button>
              </div>
            </div>
          ) : (
            /* 未配置企业微信自建固件凭据：展示友好指引卡片 + 一键模拟体验 */
            <div className="space-y-4">
              <div className="p-4 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-900 dark:text-amber-200 text-xs space-y-2">
                <div className="flex items-center gap-2 font-semibold text-amber-700 dark:text-amber-400">
                  <AlertCircle className="size-4 shrink-0" />
                  <span>{t('wecomNotConfigured')}</span>
                </div>
                <p className="text-zinc-600 dark:text-zinc-300 leading-relaxed">
                  {t('wecomNotConfiguredDesc')}
                </p>
                <div className="pt-2 border-t border-amber-500/20 text-[11px] font-mono text-zinc-500 dark:text-zinc-400">
                  {t('wecomConfigGuide')}
                </div>
              </div>

              {/* 模拟演示登录卡片 */}
              <div className="p-4 rounded-xl bg-zinc-50 dark:bg-zinc-800/60 border border-zinc-200 dark:border-zinc-700 space-y-3">
                <div className="flex items-center justify-between text-xs">
                  <span className="font-medium text-zinc-700 dark:text-zinc-300 flex items-center gap-1.5">
                    <Sparkles className="size-4 text-emerald-500" />
                    开发与测试模式 (Demo)
                  </span>
                  <span className="px-1.5 py-0.5 rounded text-[10px] bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 font-medium">
                    Ready
                  </span>
                </div>
                <p className="text-xs text-zinc-500 dark:text-zinc-400">
                  无需等待申请企业微信后台，您可以直接通过测试账号体验企业微信 SSO 登录完整链路：
                </p>

                <Button
                  type="button"
                  className="w-full bg-emerald-600 hover:bg-emerald-700 text-white shadow-sm flex items-center justify-center gap-2 text-xs h-9 cursor-pointer"
                  onClick={handleDemoLogin}
                  disabled={demoLoading || demoSuccess}
                >
                  {demoSuccess ? (
                    <>
                      <CheckCircle2 className="size-4 text-white animate-in zoom-in" />
                      {t('wecomLoginSuccess')}
                    </>
                  ) : demoLoading ? (
                    <>
                      <Loader2 className="size-4 animate-spin" />
                      正在登录...
                    </>
                  ) : (
                    <>
                      <ShieldCheck className="size-4" />
                      {t('wecomDemoLogin')}
                    </>
                  )}
                </Button>
              </div>
            </div>
          )}
        </div>

        {/* 底部安全声明 */}
        <div className="px-6 py-3 bg-zinc-50/70 dark:bg-zinc-900/70 border-t border-zinc-100 dark:border-zinc-800 flex items-center justify-between text-[11px] text-zinc-400">
          <span className="flex items-center gap-1">
            <ShieldCheck className="size-3.5 text-emerald-600 dark:text-emerald-400" />
            企业微信官方授权协议保护
          </span>
          <button
            type="button"
            onClick={onClose}
            className="hover:underline text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300 cursor-pointer"
          >
            取消
          </button>
        </div>
      </div>
    </div>
  );
}
