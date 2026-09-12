import React, { createContext, useContext, useState, useCallback, useRef, useEffect } from 'react';
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogAction,
  AlertDialogCancel,
} from '@/components/ui/alert-dialog';
import { CheckCircle2, AlertTriangle, AlertCircle, Info } from 'lucide-react';
import { useTranslations } from './I18nContext';
import clsx from 'clsx';

export type DialogVariant = 'default' | 'info' | 'success' | 'destructive' | 'warning';

export interface AlertOptions {
  title?: string;
  description: string;
  variant?: DialogVariant;
  confirmText?: string;
}

export interface ConfirmOptions {
  title?: string;
  description: string;
  variant?: DialogVariant;
  confirmText?: string;
  cancelText?: string;
}

interface ModalDialogContextType {
  alert: (messageOrOptions: string | AlertOptions) => Promise<void>;
  confirm: (messageOrOptions: string | ConfirmOptions) => Promise<boolean>;
}

const ModalDialogContext = createContext<ModalDialogContextType | null>(null);

interface DialogState {
  isOpen: boolean;
  type: 'alert' | 'confirm';
  title: string;
  description: string;
  variant: DialogVariant;
  confirmText: string;
  cancelText: string;
}

export function ModalDialogProvider({ children }: { children: React.ReactNode }) {
  const tCommon = useTranslations('Common');

  const [dialogState, setDialogState] = useState<DialogState>({
    isOpen: false,
    type: 'alert',
    title: '',
    description: '',
    variant: 'default',
    confirmText: '',
    cancelText: '',
  });

  const resolverRef = useRef<((value: any) => void) | null>(null);

  const alert = useCallback(
    (messageOrOptions: string | AlertOptions): Promise<void> => {
      return new Promise<void>((resolve) => {
        resolverRef.current = () => resolve();

        const opts =
          typeof messageOrOptions === 'string'
            ? { description: messageOrOptions }
            : messageOrOptions;

        const isSuccess =
          opts.variant === 'success' ||
          /success|成功/i.test(opts.description) ||
          /success|成功/i.test(opts.title || '');

        const isError =
          opts.variant === 'destructive' ||
          /failed|error|失败|错误|deny|denied|invalid/i.test(opts.description);

        const defaultTitle = isSuccess
          ? tCommon('successTitle') || 'Success'
          : isError
          ? tCommon('errorTitle') || 'Error'
          : tCommon('infoTitle') || 'Notice';

        setDialogState({
          isOpen: true,
          type: 'alert',
          title: opts.title || defaultTitle,
          description: opts.description,
          variant: opts.variant || (isSuccess ? 'success' : isError ? 'destructive' : 'default'),
          confirmText: opts.confirmText || tCommon('ok') || 'OK',
          cancelText: '',
        });
      });
    },
    [tCommon]
  );

  const confirm = useCallback(
    (messageOrOptions: string | ConfirmOptions): Promise<boolean> => {
      return new Promise<boolean>((resolve) => {
        resolverRef.current = (val: boolean) => resolve(val);

        const opts =
          typeof messageOrOptions === 'string'
            ? { description: messageOrOptions }
            : messageOrOptions;

        const isDelete =
          opts.variant === 'destructive' ||
          /delete|remove|uninstall|permanently|revoke|dismiss|删除|移除|卸载|永久/i.test(
            opts.description
          ) ||
          /delete|remove|删除/i.test(opts.title || '');

        const defaultTitle = isDelete
          ? tCommon('deleteConfirmTitle') || 'Confirm Delete'
          : tCommon('confirmTitle') || 'Confirm Action';

        const defaultConfirmText = isDelete
          ? tCommon('delete') || 'Delete'
          : tCommon('ok') || 'OK';

        setDialogState({
          isOpen: true,
          type: 'confirm',
          title: opts.title || defaultTitle,
          description: opts.description,
          variant: opts.variant || (isDelete ? 'destructive' : 'default'),
          confirmText: opts.confirmText || defaultConfirmText,
          cancelText: opts.cancelText || tCommon('cancel') || 'Cancel',
        });
      });
    },
    [tCommon]
  );

  // Global override for window.alert to automatically use shadcn modal
  useEffect(() => {
    const originalAlert = window.alert;
    window.alert = (msg?: any) => {
      const messageStr = typeof msg === 'string' ? msg : String(msg ?? '');
      alert(messageStr);
    };

    return () => {
      window.alert = originalAlert;
    };
  }, [alert]);

  const handleAction = () => {
    setDialogState((prev) => ({ ...prev, isOpen: false }));
    if (resolverRef.current) {
      resolverRef.current(true);
      resolverRef.current = null;
    }
  };

  const handleCancel = () => {
    setDialogState((prev) => ({ ...prev, isOpen: false }));
    if (resolverRef.current) {
      resolverRef.current(false);
      resolverRef.current = null;
    }
  };

  const getIcon = () => {
    switch (dialogState.variant) {
      case 'success':
        return (
          <div className="w-10 h-10 rounded-full bg-green-100 dark:bg-green-900/30 flex items-center justify-center text-green-600 dark:text-green-400 shrink-0">
            <CheckCircle2 className="w-5 h-5" />
          </div>
        );
      case 'destructive':
        return (
          <div className="w-10 h-10 rounded-full bg-red-100 dark:bg-red-900/30 flex items-center justify-center text-red-600 dark:text-red-400 shrink-0">
            <AlertCircle className="w-5 h-5" />
          </div>
        );
      case 'warning':
        return (
          <div className="w-10 h-10 rounded-full bg-amber-100 dark:bg-amber-900/30 flex items-center justify-center text-amber-600 dark:text-amber-400 shrink-0">
            <AlertTriangle className="w-5 h-5" />
          </div>
        );
      case 'info':
      default:
        return (
          <div className="w-10 h-10 rounded-full bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center text-primary-600 dark:text-primary-400 shrink-0">
            <Info className="w-5 h-5" />
          </div>
        );
    }
  };

  return (
    <ModalDialogContext.Provider value={{ alert, confirm }}>
      {children}
      <AlertDialog
        open={dialogState.isOpen}
        onOpenChange={(open) => {
          if (!open) handleCancel();
        }}
      >
        <AlertDialogContent className="max-w-md p-6">
          <div className="flex items-start gap-4">
            {getIcon()}
            <div className="flex-1 min-w-0">
              <AlertDialogHeader className="text-left space-y-1">
                <AlertDialogTitle className="text-base font-semibold text-gray-900 dark:text-white">
                  {dialogState.title}
                </AlertDialogTitle>
                <AlertDialogDescription className="text-sm text-gray-600 dark:text-gray-300 whitespace-pre-line leading-relaxed">
                  {dialogState.description}
                </AlertDialogDescription>
              </AlertDialogHeader>
            </div>
          </div>

          <AlertDialogFooter className="mt-6 flex sm:justify-end gap-2">
            {dialogState.type === 'confirm' && (
              <AlertDialogCancel
                onClick={handleCancel}
                className="mt-0 border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 font-medium"
              >
                {dialogState.cancelText}
              </AlertDialogCancel>
            )}
            <AlertDialogAction
              onClick={handleAction}
              className={clsx(
                'font-medium',
                dialogState.variant === 'destructive'
                  ? 'bg-red-600 hover:bg-red-700 text-white focus:ring-red-500'
                  : dialogState.variant === 'success'
                  ? 'bg-green-600 hover:bg-green-700 text-white focus:ring-green-500'
                  : 'bg-primary-600 hover:bg-primary-700 text-white focus:ring-primary-500'
              )}
            >
              {dialogState.confirmText}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </ModalDialogContext.Provider>
  );
}

export function useModalDialog() {
  const context = useContext(ModalDialogContext);
  if (!context) {
    throw new Error('useModalDialog must be used within a ModalDialogProvider');
  }
  return context;
}
