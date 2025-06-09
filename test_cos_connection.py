#!/usr/bin/env python3
"""
测试腾讯云COS连接和数据同步
"""

import json
import sys

try:
    from qcloud_cos import CosConfig
    from qcloud_cos import CosS3Client
except ImportError:
    print("需要安装 cos-python-sdk-v5")
    print("运行: pip install cos-python-sdk-v5")
    sys.exit(1)

# 从配置文件读取配置
config_file = "/Users/jingbofan/Library/Application Support/clippy/storage_config.json"

try:
    with open(config_file, 'r') as f:
        config = json.load(f)
        
    backend = config['backend']
    
    # 提取COS配置
    bucket = backend['bucket']
    endpoint = backend['endpoint']
    secret_id = backend['secret_id']
    secret_key = backend['secret_key']
    
    # 从endpoint提取region
    # https://cos.ap-chengdu.myqcloud.com -> ap-chengdu
    region = endpoint.split('.')[1] if '.' in endpoint else 'ap-chengdu'
    
    print(f"测试COS连接:")
    print(f"存储桶: {bucket}")
    print(f"区域: {region}")
    print(f"端点: {endpoint}")
    print()
    
    # 创建配置对象
    cos_config = CosConfig(Region=region, SecretId=secret_id, SecretKey=secret_key)
    client = CosS3Client(cos_config)
    
    # 测试连接
    try:
        response = client.head_bucket(Bucket=bucket)
        print("✅ COS连接成功!")
        print()
        
        # 检查是否有clippy数据
        user_id = "default_user"
        
        # 列出oplog目录
        oplog_prefix = f"{user_id}/oplog/"
        print(f"检查oplog目录: {oplog_prefix}")
        
        try:
            response = client.list_objects(
                Bucket=bucket,
                Prefix=oplog_prefix,
                MaxKeys=10
            )
            
            if 'Contents' in response:
                print(f"✅ 找到 {len(response['Contents'])} 个oplog文件:")
                for obj in response['Contents']:
                    print(f"  - {obj['Key']} (大小: {obj['Size']} 字节)")
            else:
                print("❌ 未找到oplog文件")
                
        except Exception as e:
            print(f"❌ 列出oplog文件失败: {e}")
            
        # 检查快照目录
        snapshot_prefix = f"{user_id}/snapshots/"
        print(f"\n检查快照目录: {snapshot_prefix}")
        
        try:
            response = client.list_objects(
                Bucket=bucket,
                Prefix=snapshot_prefix,
                MaxKeys=10
            )
            
            if 'Contents' in response:
                print(f"✅ 找到 {len(response['Contents'])} 个快照文件:")
                for obj in response['Contents']:
                    print(f"  - {obj['Key']} (大小: {obj['Size']} 字节)")
            else:
                print("❌ 未找到快照文件")
                
        except Exception as e:
            print(f"❌ 列出快照文件失败: {e}")
        
    except Exception as e:
        print(f"❌ COS连接失败: {e}")
        
except FileNotFoundError:
    print(f"❌ 配置文件未找到: {config_file}")
except json.JSONDecodeError:
    print(f"❌ 配置文件格式错误: {config_file}")
except Exception as e:
    print(f"❌ 读取配置失败: {e}") 