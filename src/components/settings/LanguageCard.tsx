import { Globe } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useI18n, type Locale } from '@/i18n/provider';

export function LanguageCard() {
  const { t, locale, setLocale } = useI18n();

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-2">
          <Globe className="size-5" />
          <CardTitle>{t('settings.language')}</CardTitle>
        </div>
        <CardDescription>{t('settings.languageDescription')}</CardDescription>
      </CardHeader>
      <CardContent>
        <Select value={locale} onValueChange={(value) => setLocale(value as Locale)}>
          <SelectTrigger className="w-56">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="system">{t('settings.localeSystem')}</SelectItem>
            <SelectItem value="zh-CN">{t('settings.localeZhCN')}</SelectItem>
            <SelectItem value="en">{t('settings.localeEn')}</SelectItem>
          </SelectContent>
        </Select>
      </CardContent>
    </Card>
  );
}
