## 原理
fincore工具通过mmap映射文件到内存获取fd，采用mincore获取文件加载到cache中的pages。