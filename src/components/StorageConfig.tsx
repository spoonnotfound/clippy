import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Label } from './ui/label';
import { Card, CardContent, CardHeader, CardTitle } from './ui/card';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from './ui/select';
import { Alert, AlertDescription } from './ui/alert';
import { Tabs, TabsContent, TabsList, TabsTrigger } from './ui/tabs';
import { Badge } from './ui/badge';
import { CheckCircle, XCircle, AlertCircle, Loader2 } from 'lucide-react';

type StorageBackend = 
  | { type: "FileSystem"; root_path: string }
  | { type: "S3"; bucket: string; region: string; access_key_id: string; secret_access_key: string; endpoint?: string }
  | { type: "S3Compatible"; bucket: string; endpoint: string; access_key_id: string; secret_access_key: string; region?: string }
  | { type: "Oss"; bucket: string; endpoint: string; access_key_id: string; access_key_secret: string }
  | { type: "Cos"; bucket: string; endpoint: string; secret_id: string; secret_key: string }
  | { type: "AzBlob"; container: string; account_name: string; account_key: string };

interface StorageConfig {
  backend: StorageBackend;
  retry_attempts: number;
  timeout_seconds: number;
}

interface SyncStatus {
  item_count: number;
  is_syncing: boolean;
}

export const StorageConfig: React.FC = () => {
  const [config, setConfig] = useState<StorageConfig>({
    backend: { type: "FileSystem", root_path: './clippy_sync_data' },
    retry_attempts: 3,
    timeout_seconds: 30,
  });
  
  const [backendType, setBackendType] = useState<string>('FileSystem');
  const [isLoading, setIsLoading] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [message, setMessage] = useState<{ type: 'success' | 'error' | 'info'; text: string } | null>(null);
  const [syncStatus, setSyncStatus] = useState<SyncStatus | null>(null);

  useEffect(() => {
    loadConfig();
    loadSyncStatus();
  }, []);

  const loadConfig = async () => {
    try {
      const currentConfig = await invoke<any>('get_storage_config');
      setConfig(currentConfig);
      
      // 确定当前的后端类型
      const backend = currentConfig.backend;
      setBackendType(backend.type);
    } catch (error) {
      console.error('Failed to load config:', error);
      setMessage({ type: 'error', text: '加载配置失败' });
    }
  };

  const loadSyncStatus = async () => {
    try {
      const status = await invoke<SyncStatus>('get_sync_status');
      setSyncStatus(status);
    } catch (error) {
      console.error('Failed to load sync status:', error);
    }
  };

  const saveConfig = async () => {
    setIsLoading(true);
    setMessage(null);
    
    try {
      await invoke('configure_storage', { storageConfig: config });
      setMessage({ type: 'success', text: '配置保存成功！重启应用后生效。' });
      await loadSyncStatus();
    } catch (error) {
      setMessage({ type: 'error', text: `保存失败: ${error}` });
    } finally {
      setIsLoading(false);
    }
  };

  const testConnection = async () => {
    setIsTesting(true);
    setMessage(null);
    
    try {
      const result = await invoke<string>('test_storage_connection', { storageConfig: config });
      setMessage({ type: 'success', text: result });
    } catch (error) {
      setMessage({ type: 'error', text: `${error}` });
    } finally {
      setIsTesting(false);
    }
  };

  const syncNow = async () => {
    try {
      await invoke('sync_now');
      setMessage({ type: 'success', text: '同步完成！' });
      await loadSyncStatus();
    } catch (error) {
      setMessage({ type: 'error', text: `同步失败: ${error}` });
    }
  };

  const updateBackend = (newBackendType: string) => {
    setBackendType(newBackendType);
    
    // 创建新的后端配置
    let newBackend: StorageBackend;
    
    switch (newBackendType) {
      case 'FileSystem':
        newBackend = { type: "FileSystem", root_path: './clippy_sync_data' };
        break;
      case 'S3':
        newBackend = {
          type: "S3",
          bucket: '',
          region: 'us-east-1',
          access_key_id: '',
          secret_access_key: '',
          endpoint: undefined,
        };
        break;
      case 'S3Compatible':
        newBackend = {
          type: "S3Compatible",
          bucket: '',
          endpoint: '',
          access_key_id: '',
          secret_access_key: '',
          region: 'us-east-1',
        };
        break;
      case 'Oss':
        newBackend = {
          type: "Oss",
          bucket: '',
          endpoint: 'https://oss-cn-hangzhou.aliyuncs.com',
          access_key_id: '',
          access_key_secret: '',
        };
        break;
      case 'Cos':
        newBackend = {
          type: "Cos",
          bucket: '',
          endpoint: 'https://cos.ap-guangzhou.myqcloud.com',
          secret_id: '',
          secret_key: '',
        };
        break;
      case 'AzBlob':
        newBackend = {
          type: "AzBlob",
          container: '',
          account_name: '',
          account_key: '',
        };
        break;
      default:
        newBackend = { type: "FileSystem", root_path: './clippy_sync_data' };
    }
    
    setConfig(prev => ({ ...prev, backend: newBackend }));
  };

  const updateBackendField = (field: string, value: string) => {
    setConfig(prev => ({
      ...prev,
      backend: {
        ...prev.backend,
        [field]: value === '' ? undefined : value,
      },
    }));
  };

  const renderBackendFields = () => {
    const backend = config.backend;
    
    if (backend.type !== backendType) return null;

    // Helper function to get field value safely
    const getFieldValue = (field: string): string => {
      return (backend as any)[field] || '';
    };

    switch (backend.type) {
      case 'FileSystem':
        return (
          <div className="space-y-4">
            <div>
              <Label htmlFor="root_path">存储路径</Label>
              <Input
                id="root_path"
                value={getFieldValue('root_path')}
                onChange={(e) => updateBackendField('root_path', e.target.value)}
                placeholder="./clippy_sync_data"
              />
            </div>
          </div>
        );

      case 'S3':
        return (
          <div className="space-y-4">
            <div>
              <Label htmlFor="bucket">存储桶名称</Label>
              <Input
                id="bucket"
                value={getFieldValue('bucket')}
                onChange={(e) => updateBackendField('bucket', e.target.value)}
                placeholder="my-clippy-bucket"
              />
            </div>
            <div>
              <Label htmlFor="region">区域</Label>
              <Input
                id="region"
                value={getFieldValue('region')}
                onChange={(e) => updateBackendField('region', e.target.value)}
                placeholder="us-east-1"
              />
            </div>
            <div>
              <Label htmlFor="access_key_id">访问密钥ID</Label>
              <Input
                id="access_key_id"
                value={getFieldValue('access_key_id')}
                onChange={(e) => updateBackendField('access_key_id', e.target.value)}
                placeholder="AKIAIOSFODNN7EXAMPLE"
              />
            </div>
            <div>
              <Label htmlFor="secret_access_key">私有访问密钥</Label>
              <Input
                id="secret_access_key"
                type="password"
                value={getFieldValue('secret_access_key') === '***' ? '' : getFieldValue('secret_access_key')}
                onChange={(e) => updateBackendField('secret_access_key', e.target.value)}
                placeholder={getFieldValue('secret_access_key') === '***' ? '已设置 (输入新值以更改)' : ''}
              />
            </div>
            <div>
              <Label htmlFor="endpoint">自定义端点 (可选)</Label>
              <Input
                id="endpoint"
                value={getFieldValue('endpoint')}
                onChange={(e) => updateBackendField('endpoint', e.target.value)}
                placeholder="https://s3.amazonaws.com"
              />
            </div>
          </div>
        );

      case 'S3Compatible':
        return (
          <div className="space-y-4">
            <div>
              <Label htmlFor="bucket">存储桶名称</Label>
              <Input
                id="bucket"
                value={getFieldValue('bucket')}
                onChange={(e) => updateBackendField('bucket', e.target.value)}
                placeholder="clippy"
              />
            </div>
            <div>
              <Label htmlFor="endpoint">端点地址</Label>
              <Input
                id="endpoint"
                value={getFieldValue('endpoint')}
                onChange={(e) => updateBackendField('endpoint', e.target.value)}
                placeholder="http://localhost:9000"
              />
            </div>
            <div>
              <Label htmlFor="access_key_id">访问密钥</Label>
              <Input
                id="access_key_id"
                value={getFieldValue('access_key_id')}
                onChange={(e) => updateBackendField('access_key_id', e.target.value)}
                placeholder="admin"
              />
            </div>
            <div>
              <Label htmlFor="secret_access_key">私有密钥</Label>
              <Input
                id="secret_access_key"
                type="password"
                value={getFieldValue('secret_access_key') === '***' ? '' : getFieldValue('secret_access_key')}
                onChange={(e) => updateBackendField('secret_access_key', e.target.value)}
                placeholder={getFieldValue('secret_access_key') === '***' ? '已设置 (输入新值以更改)' : 'password123'}
              />
            </div>
            <div>
              <Label htmlFor="region">区域 (可选)</Label>
              <Input
                id="region"
                value={getFieldValue('region')}
                onChange={(e) => updateBackendField('region', e.target.value)}
                placeholder="us-east-1"
              />
            </div>
          </div>
        );

      case 'Oss':
        return (
          <div className="space-y-4">
            <div>
              <Label htmlFor="bucket">存储桶名称</Label>
              <Input
                id="bucket"
                value={getFieldValue('bucket')}
                onChange={(e) => updateBackendField('bucket', e.target.value)}
                placeholder="my-clippy-bucket"
              />
            </div>
            <div>
              <Label htmlFor="endpoint">端点地址</Label>
              <Input
                id="endpoint"
                value={getFieldValue('endpoint')}
                onChange={(e) => updateBackendField('endpoint', e.target.value)}
                placeholder="https://oss-cn-hangzhou.aliyuncs.com"
              />
            </div>
            <div>
              <Label htmlFor="access_key_id">访问密钥ID</Label>
              <Input
                id="access_key_id"
                value={getFieldValue('access_key_id')}
                onChange={(e) => updateBackendField('access_key_id', e.target.value)}
                placeholder="LTAI4G***"
              />
            </div>
            <div>
              <Label htmlFor="access_key_secret">访问密钥</Label>
              <Input
                id="access_key_secret"
                type="password"
                value={getFieldValue('access_key_secret') === '***' ? '' : getFieldValue('access_key_secret')}
                onChange={(e) => updateBackendField('access_key_secret', e.target.value)}
                placeholder={getFieldValue('access_key_secret') === '***' ? '已设置 (输入新值以更改)' : ''}
              />
            </div>
          </div>
        );

      case 'Cos':
        return (
          <div className="space-y-4">
            <div>
              <Label htmlFor="bucket">存储桶名称</Label>
              <Input
                id="bucket"
                value={getFieldValue('bucket')}
                onChange={(e) => updateBackendField('bucket', e.target.value)}
                placeholder="my-clippy-bucket-1234567890"
              />
            </div>
            <div>
              <Label htmlFor="endpoint">端点地址</Label>
              <Input
                id="endpoint"
                value={getFieldValue('endpoint')}
                onChange={(e) => updateBackendField('endpoint', e.target.value)}
                placeholder="https://cos.ap-guangzhou.myqcloud.com"
              />
            </div>
            <div>
              <Label htmlFor="secret_id">密钥ID</Label>
              <Input
                id="secret_id"
                value={getFieldValue('secret_id')}
                onChange={(e) => updateBackendField('secret_id', e.target.value)}
                placeholder="AKIDrAr7***"
              />
            </div>
            <div>
              <Label htmlFor="secret_key">密钥</Label>
              <Input
                id="secret_key"
                type="password"
                value={getFieldValue('secret_key') === '***' ? '' : getFieldValue('secret_key')}
                onChange={(e) => updateBackendField('secret_key', e.target.value)}
                placeholder={getFieldValue('secret_key') === '***' ? '已设置 (输入新值以更改)' : ''}
              />
            </div>
          </div>
        );

      case 'AzBlob':
        return (
          <div className="space-y-4">
            <div>
              <Label htmlFor="container">容器名称</Label>
              <Input
                id="container"
                value={getFieldValue('container')}
                onChange={(e) => updateBackendField('container', e.target.value)}
                placeholder="clippy"
              />
            </div>
            <div>
              <Label htmlFor="account_name">存储账户名</Label>
              <Input
                id="account_name"
                value={getFieldValue('account_name')}
                onChange={(e) => updateBackendField('account_name', e.target.value)}
                placeholder="mystorageaccount"
              />
            </div>
            <div>
              <Label htmlFor="account_key">账户密钥</Label>
              <Input
                id="account_key"
                type="password"
                value={getFieldValue('account_key') === '***' ? '' : getFieldValue('account_key')}
                onChange={(e) => updateBackendField('account_key', e.target.value)}
                placeholder={getFieldValue('account_key') === '***' ? '已设置 (输入新值以更改)' : ''}
              />
            </div>
          </div>
        );

      default:
        return null;
    }
  };

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            同步配置
            {syncStatus && (
              <Badge variant={syncStatus.item_count > 0 ? "default" : "secondary"}>
                {syncStatus.item_count} 个已同步项目
              </Badge>
            )}
          </CardTitle>
        </CardHeader>
        <CardContent>
          <Tabs defaultValue="config" className="w-full">
            <TabsList>
              <TabsTrigger value="config">存储配置</TabsTrigger>
              <TabsTrigger value="sync">同步控制</TabsTrigger>
            </TabsList>
            
            <TabsContent value="config" className="space-y-6">
              <div className="space-y-4">
                <div>
                  <Label htmlFor="backend-type">存储后端类型</Label>
                  <Select value={backendType} onValueChange={updateBackend}>
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="FileSystem">本地文件系统</SelectItem>
                      <SelectItem value="S3Compatible">MinIO / S3兼容</SelectItem>
                      <SelectItem value="S3">Amazon S3</SelectItem>
                      <SelectItem value="Oss">阿里云 OSS</SelectItem>
                      <SelectItem value="Cos">腾讯云 COS</SelectItem>
                      <SelectItem value="AzBlob">Azure Blob Storage</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                {renderBackendFields()}

                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <Label htmlFor="retry_attempts">重试次数</Label>
                    <Input
                      id="retry_attempts"
                      type="number"
                      value={config.retry_attempts}
                      onChange={(e) => setConfig(prev => ({ ...prev, retry_attempts: parseInt(e.target.value) || 3 }))}
                      min="1"
                      max="10"
                    />
                  </div>
                  <div>
                    <Label htmlFor="timeout_seconds">超时时间 (秒)</Label>
                    <Input
                      id="timeout_seconds"
                      type="number"
                      value={config.timeout_seconds}
                      onChange={(e) => setConfig(prev => ({ ...prev, timeout_seconds: parseInt(e.target.value) || 30 }))}
                      min="5"
                      max="300"
                    />
                  </div>
                </div>
              </div>

              <div className="flex gap-2">
                <Button onClick={testConnection} disabled={isTesting} variant="outline">
                  {isTesting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                  测试连接
                </Button>
                <Button onClick={saveConfig} disabled={isLoading}>
                  {isLoading && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                  保存配置
                </Button>
              </div>
            </TabsContent>
            
            <TabsContent value="sync" className="space-y-6">
              <div className="space-y-4">
                {syncStatus && (
                  <div className="grid grid-cols-2 gap-4">
                    <Card>
                      <CardContent className="p-4">
                        <div className="text-2xl font-bold">{syncStatus.item_count}</div>
                        <div className="text-sm text-muted-foreground">已同步项目</div>
                      </CardContent>
                    </Card>
                    <Card>
                      <CardContent className="p-4">
                        <div className="flex items-center gap-2">
                          {syncStatus.is_syncing ? (
                            <div className="flex items-center gap-2 text-blue-600">
                              <Loader2 className="h-4 w-4 animate-spin" />
                              <span>同步中</span>
                            </div>
                          ) : (
                            <div className="flex items-center gap-2 text-green-600">
                              <CheckCircle className="h-4 w-4" />
                              <span>已同步</span>
                            </div>
                          )}
                        </div>
                        <div className="text-sm text-muted-foreground">同步状态</div>
                      </CardContent>
                    </Card>
                  </div>
                )}
                
                <div>
                  <Button onClick={syncNow} className="w-full">
                    立即同步
                  </Button>
                </div>
              </div>
            </TabsContent>
          </Tabs>

          {message && (
            <Alert className={`mt-4 ${
              message.type === 'success' ? 'border-green-500' : 
              message.type === 'error' ? 'border-red-500' : 'border-blue-500'
            }`}>
              {message.type === 'success' && <CheckCircle className="h-4 w-4" />}
              {message.type === 'error' && <XCircle className="h-4 w-4" />}
              {message.type === 'info' && <AlertCircle className="h-4 w-4" />}
              <AlertDescription>{message.text}</AlertDescription>
            </Alert>
          )}
        </CardContent>
      </Card>
    </div>
  );
}; 