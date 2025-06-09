import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from './ui/card';
import { Alert, AlertDescription } from './ui/alert';
import { Badge } from './ui/badge';
import { ExternalLink, Server, Cloud } from 'lucide-react';

export const ConfigGuide: React.FC = () => {
  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Cloud className="h-5 w-5" />
            存储后端选择指南
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <Alert>
            <AlertDescription>
              选择合适的存储后端来实现多设备剪切板同步。不同后端适用于不同的使用场景。
            </AlertDescription>
          </Alert>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {/* 本地文件系统 */}
            <Card>
              <CardContent className="p-4">
                <div className="flex items-center justify-between mb-2">
                  <h3 className="font-medium">本地文件系统</h3>
                  <Badge variant="secondary">开发测试</Badge>
                </div>
                <p className="text-sm text-gray-600 mb-3">
                  适用于单机使用或开发测试，不支持多设备同步
                </p>
                <div className="text-xs text-gray-500">
                  <div>✅ 无需网络连接</div>
                  <div>✅ 简单快速</div>
                  <div>❌ 不支持多设备同步</div>
                </div>
              </CardContent>
            </Card>

            {/* MinIO */}
            <Card>
              <CardContent className="p-4">
                <div className="flex items-center justify-between mb-2">
                  <h3 className="font-medium">MinIO (S3兼容)</h3>
                  <Badge variant="default">推荐</Badge>
                </div>
                <p className="text-sm text-gray-600 mb-3">
                  自建私有云存储，完全控制数据，支持Docker部署
                </p>
                <div className="text-xs text-gray-500">
                  <div>✅ 私有部署</div>
                  <div>✅ 完全控制</div>
                  <div>✅ 免费使用</div>
                  <div>⚠️ 需要服务器</div>
                </div>
              </CardContent>
            </Card>

            {/* Amazon S3 */}
            <Card>
              <CardContent className="p-4">
                <div className="flex items-center justify-between mb-2">
                  <h3 className="font-medium">Amazon S3</h3>
                  <Badge variant="outline">云服务</Badge>
                </div>
                <p className="text-sm text-gray-600 mb-3">
                  AWS官方存储服务，稳定可靠，全球部署
                </p>
                <div className="text-xs text-gray-500">
                  <div>✅ 高可用性</div>
                  <div>✅ 全球节点</div>
                  <div>💰 按使用付费</div>
                  <div>⚠️ 需要AWS账户</div>
                </div>
              </CardContent>
            </Card>

            {/* 阿里云OSS */}
            <Card>
              <CardContent className="p-4">
                <div className="flex items-center justify-between mb-2">
                  <h3 className="font-medium">阿里云 OSS</h3>
                  <Badge variant="outline">国内优化</Badge>
                </div>
                <p className="text-sm text-gray-600 mb-3">
                  国内访问速度快，适合中国用户
                </p>
                <div className="text-xs text-gray-500">
                  <div>✅ 国内访问快</div>
                  <div>✅ 中文支持</div>
                  <div>💰 按使用付费</div>
                  <div>⚠️ 需要阿里云账户</div>
                </div>
              </CardContent>
            </Card>
          </div>
        </CardContent>
      </Card>

      {/* MinIO 快速部署指南 */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Server className="h-5 w-5" />
            MinIO 快速部署 (推荐)
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <Alert>
            <AlertDescription>
              MinIO 是最简单的部署方案，只需要 Docker 就能快速搭建私有存储。
            </AlertDescription>
          </Alert>

          <div className="space-y-3">
            <div>
              <h4 className="font-medium mb-2">1. 启动 MinIO 服务器</h4>
              <div className="bg-gray-100 p-3 rounded-md font-mono text-sm overflow-x-auto">
                docker run -p 9000:9000 -p 9001:9001 \<br/>
                &nbsp;&nbsp;-e "MINIO_ROOT_USER=admin" \<br/>
                &nbsp;&nbsp;-e "MINIO_ROOT_PASSWORD=password123" \<br/>
                &nbsp;&nbsp;quay.io/minio/minio server /data --console-address ":9001"
              </div>
            </div>

            <div>
              <h4 className="font-medium mb-2">2. 访问 MinIO 控制台</h4>
              <div className="flex items-center gap-2">
                <span className="text-sm">打开浏览器访问:</span>
                <a 
                  href="http://localhost:9001" 
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-blue-600 hover:text-blue-800 flex items-center gap-1"
                >
                  http://localhost:9001
                  <ExternalLink className="h-3 w-3" />
                </a>
              </div>
              <div className="text-sm text-gray-600 mt-1">
                用户名: <code>admin</code>, 密码: <code>password123</code>
              </div>
            </div>

            <div>
              <h4 className="font-medium mb-2">3. 创建存储桶</h4>
              <div className="text-sm text-gray-600">
                在 MinIO 控制台中创建一个名为 <code>clippy</code> 的存储桶
              </div>
            </div>

            <div>
              <h4 className="font-medium mb-2">4. 配置 Clippy</h4>
              <div className="text-sm text-gray-600">
                在上方"存储配置"页面选择 "MinIO / S3兼容"，并填入以下信息：
              </div>
              <div className="bg-gray-100 p-3 rounded-md text-sm mt-2">
                <div><strong>存储桶名称:</strong> clippy</div>
                <div><strong>端点地址:</strong> http://localhost:9000</div>
                <div><strong>访问密钥:</strong> admin</div>
                <div><strong>私有密钥:</strong> password123</div>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* 安全提示 */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <span className="text-yellow-600">⚠️</span>
            安全提示
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-2 text-sm">
            <div>• 生产环境请修改默认的用户名和密码</div>
            <div>• 建议启用 HTTPS 访问</div>
            <div>• 定期备份重要数据</div>
            <div>• 不要在配置中硬编码敏感信息</div>
            <div>• 考虑实现端到端加密（未来版本将支持）</div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}; 