# pip install boto3
import boto3
from datetime import datetime, UTC
from botocore.client import Config

# === Configuration ===
S3_ENDPOINT = 'http://localhost:8000'  # Replace with your S3-compatible server
ACCESS_KEY = 'your-access-key'
SECRET_KEY = 'your-secret-key'
BUCKET_NAME = 'test'
OBJECT_KEY = 'now.txt'

# === Setup client ===
s3 = boto3.client(
    's3',
    endpoint_url=S3_ENDPOINT,
    aws_access_key_id=ACCESS_KEY,
    aws_secret_access_key=SECRET_KEY,
    config=Config(signature_version='s3v4'),
    region_name='us-east-1'
)

# === Create Bucket ===
try:
    s3.create_bucket(Bucket=BUCKET_NAME)
    print(f"Bucket '{BUCKET_NAME}' created.")
except s3.exceptions.BucketAlreadyOwnedByYou:
    print(f"Bucket '{BUCKET_NAME}' already exists (owned by you).")

# === Write file ===
timestamp =  datetime.now(UTC).isoformat()
s3.put_object(Bucket=BUCKET_NAME, Key=OBJECT_KEY, Body=timestamp.encode('utf-8'))
print(f"Object '{OBJECT_KEY}' written with timestamp: {timestamp}")

# === Read file ===
response = s3.get_object(Bucket=BUCKET_NAME, Key=OBJECT_KEY)
content = response['Body'].read().decode('utf-8')
print(f"Read from '{OBJECT_KEY}': {content}")


# === List all buckets ===
print("Buckets:")
buckets = s3.list_buckets()
for bucket in buckets['Buckets']:
    print(f"{bucket['Name']}: {bucket!r}")

# === List objects in bucket ===
print(f"Objects in bucket '{BUCKET_NAME}':")
objects = s3.list_objects_v2(Bucket=BUCKET_NAME)
for obj in objects.get('Contents', []):
    print(f"{obj['Key']}: {obj!r}")
input()

# === Delete file ===
s3.delete_object(Bucket=BUCKET_NAME, Key=OBJECT_KEY)
print(f"Deleted object '{OBJECT_KEY}'.")

# === Delete bucket ===
s3.delete_bucket(Bucket=BUCKET_NAME)
print(f"Deleted bucket '{BUCKET_NAME}'.")
