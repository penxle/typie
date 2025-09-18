#!/bin/bash
set -euo pipefail

# Configuration
ARCH=arm64
K8S_VERSION=1.34
CONTAINERD_VERSION=2.1.4
RUNC_VERSION=1.3.1
CNI_PLUGINS_VERSION=1.8.0

# Create sysctl configuration for Kubernetes
echo "Creating sysctl configuration..."
cat > /etc/sysctl.d/99-kubernetes.conf << 'EOF'
net.ipv4.ip_forward = 1
EOF

# Install required packages
echo "Installing required packages..."
apt-get update
apt-get install -y apt-transport-https ca-certificates curl gpg

# Apply sysctl settings
sysctl --system

# Install containerd
echo "Installing containerd ${CONTAINERD_VERSION}..."
curl -fsSL "https://github.com/containerd/containerd/releases/download/v${CONTAINERD_VERSION}/containerd-${CONTAINERD_VERSION}-linux-${ARCH}.tar.gz" | tar Cxzv /usr/local
curl -fsSL https://raw.githubusercontent.com/containerd/containerd/main/containerd.service -o /etc/systemd/system/containerd.service

# Install runc
echo "Installing runc ${RUNC_VERSION}..."
curl -fsSL "https://github.com/opencontainers/runc/releases/download/v${RUNC_VERSION}/runc.${ARCH}" -o /usr/local/sbin/runc
chmod 755 /usr/local/sbin/runc

# Install CNI plugins
echo "Installing CNI plugins ${CNI_PLUGINS_VERSION}..."
mkdir -p /opt/cni/bin
curl -fsSL "https://github.com/containernetworking/plugins/releases/download/v${CNI_PLUGINS_VERSION}/cni-plugins-linux-${ARCH}-v${CNI_PLUGINS_VERSION}.tgz" | tar Cxzv /opt/cni/bin

# Configure containerd
echo "Configuring containerd..."
mkdir -p /etc/containerd
containerd config default > /etc/containerd/config.toml
sed -i "/\[plugins\.'io\.containerd\.cri\.v1\.runtime'\.containerd\.runtimes\.runc\.options\]/a\            SystemdCgroup = true" /etc/containerd/config.toml

# Add Kubernetes repository
echo "Adding Kubernetes ${K8S_VERSION} repository..."
curl -fsSL "https://pkgs.k8s.io/core:/stable:/v${K8S_VERSION}/deb/Release.key" | gpg --dearmor -o /etc/apt/keyrings/kubernetes-apt-keyring.gpg
echo "deb [signed-by=/etc/apt/keyrings/kubernetes-apt-keyring.gpg] https://pkgs.k8s.io/core:/stable:/v${K8S_VERSION}/deb/ /" > /etc/apt/sources.list.d/kubernetes.list

# Install Kubernetes components
echo "Installing Kubernetes components..."
apt-get update
apt-get install -y kubelet kubeadm kubectl
apt-mark hold kubelet kubeadm kubectl

# Start services
echo "Starting services..."
systemctl daemon-reload
systemctl enable --now containerd
systemctl enable --now kubelet

echo "Kubernetes installation completed!"
