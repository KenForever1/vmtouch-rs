[TOC]

## 介绍

该项目采用rust语言实现了[fincore工具](./fincore/README.md)和[vmtouch工具](./vmtouch/README.md)。

fincore工具采用mincore获取文件加载到cache中的pages。fincore只能对单文件以及*文件进行统计，不能递归统计目录。相比之下vmtouch更加强大，可以统计目录中文件加载到内存中的pages，也可以对内存中page进行锁住(lock)、回收(evict)等。

## fincore

### fincore工具示例
```bash
cd fincore
cargo build 
```

```bash
$ cargo run --package fincore -- --help

a mem tool named fincore

Usage: fincore [OPTIONS] <PATHES>...

Arguments:
  <PATHES>...  Path to the file

Options:
  -p, --pages        Print page index
  -s, --summarize    When comparing multiple files, print a summary report
  -o, --only-cached  Only print cached pages
  -h, --help         Print help
  -V, --version      Print version
```

```bash
$ cargo run --package fincore -- * -p

path: ../Cargo.lock, cached_pages: 0
path: ../Cargo.toml, cached_pages: 0
path: ../README.md, cached_pages: 0
+---------------+-----------+-------------+--------+-------------+-------------------+
| path          | file_size | total_pages | cached | cached_size | cached_percent    |
+---------------+-----------+-------------+--------+-------------+-------------------+
| ../Cargo.lock | 9609      | 3           | 1      | 4096        | 33.33333333333333 |
+---------------+-----------+-------------+--------+-------------+-------------------+
| ../Cargo.toml | 45        | 1           | 1      | 4096        | 100               |
+---------------+-----------+-------------+--------+-------------+-------------------+
| ../README.md  | 603       | 1           | 1      | 4096        | 100               |
+---------------+-----------+-------------+--------+-------------+-------------------+
total cached size: 12288 byte
```

其灵感来自[fincore](https://github.com/david415/linux-ftools)。

## vmtouch

### vmtouch工具示例

```bash
$ cargo run --package vmtouch -- -l
Locking ./a
Resident Pages: 1 1 4096 4096 100.000
```

其灵感来自[vmtouch](https://github.com/hoytech/vmtouch)。

## 工具应用

+ [我的内存去哪了？](https://www.cnblogs.com/t-bar/p/17359545.html)
+ [vmtouch-the Virtual Memory Toucher](https://hoytech.com/vmtouch/)