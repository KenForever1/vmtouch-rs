
## 多掌握一个兵器之vmtouch

vmtouch是一个unix系统下的工具，通过它，我们可以了解文件系统中cache的情况，然后我们就可以去控制cache，比如锁住缓存（lock）、回收（evict）。
在某些程序中，我们的数据文件会反复的读取，或者加载cache中，导致内存大量增长，通过vmtouch工具就可以观察是哪些文件占用了。

来看几个例子哈。
1. 查看/bin文件目录下有哪些文件在cache里面？
```bash
$ vmtouch /bin/
           Files: 92
     Directories: 1
  Resident Pages: 348/1307  1M/5M  26.6%
         Elapsed: 0.003426 seconds
```
1. big-dataset.txt这个文件在内存里面没有?

```bash
$ vmtouch -v big-dataset.txt
big-dataset.txt
[                                                            ] 0/42116

           Files: 1
     Directories: 0
  Resident Pages: 0/42116  0/164M  0%
         Elapsed: 0.005182 seconds
```
用tail命令可以把文件加载到内存中，
```bash
$ tail -n 10000 big-dataset.txt > /dev/null
```
再来看看文件在内存中的情况：
```bash
$ vmtouch -v big-dataset.txt
big-dataset.txt
[                                                    oOOOOOOO] 4950/42116

           Files: 1
     Directories: 0
  Resident Pages: 4950/42116  19M/164M  11.8%
         Elapsed: 0.006706 seconds
```
有4950 pages在memory中了，也就是tail命令加载的文件最后的10000行数据。

3. 如何通过touch操作把文件剩余部分加载到内存中？
-t就是touch，原理是把文件mmap映射一块内存区域，然后安装page大小间隔读取访问一遍。因为内存加载最小的粒度是page（一般4kb），所以访问这一遍，文件就都加载到内存中去了。

```bash
$ vmtouch -vt big-dataset.txt
big-dataset.txt
[OOo                                                 oOOOOOOO] 6887/42116
[OOOOOOOOo                                           oOOOOOOO] 10631/42116
[OOOOOOOOOOOOOOo                                     oOOOOOOO] 15351/42116
[OOOOOOOOOOOOOOOOOOOOOo                              oOOOOOOO] 19719/42116
[OOOOOOOOOOOOOOOOOOOOOOOOOOOo                        oOOOOOOO] 24183/42116
[OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOo                  oOOOOOOO] 28615/42116
[OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOo              oOOOOOOO] 31415/42116
[OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOo      oOOOOOOO] 36775/42116
[OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOo  oOOOOOOO] 39431/42116
[OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOO] 42116/42116

           Files: 1
     Directories: 0
   Touched Pages: 42116 (164M)
         Elapsed: 12.107 seconds
```

4. 有3个大文件：a.txt、b.txt 和 c.txt，但内存中只能容纳其中的 2 个大文件。如果内存中有 a.txt 和 b.txt，但现在想使用 b.txt 和 c.txt，我们可以开始加载 c.txt，但随后系统会从 a.txt （这我们想要）和 b.txt （我们不想要）中逐出页面。
   这时候可以通过vmtouch给系统提建议，也就是控制系统，把a.txt回收了(evict), 给c.txt腾出位置。
```bash
$ vmtouch -ve a.txt
Evicting a.txt

           Files: 1
     Directories: 0
   Evicted Pages: 42116 (164M)
         Elapsed: 0.076824 seconds
```

5. 把目录下所有的文件都锁到物理内存中？
```bash
vmtouch -dl /var/www/htdocs/critical/
```
## 再多思考一丢丢
1. touch和lock的区别？
touch: 只加载文件到内存中，未锁定，可能会被交换出去。适用于希望加速访问但不需要保证内存驻留的情况。
lock: 加载并锁定文件到内存中，不会被交换出去。适用于必须保证数据始终在内存中的高性能场景。
实现上，lock调用的mlock函数，touch是把每个页面的第一个位置都读取一下。

对实际应用感兴趣的朋友还可以看看这篇文章，[我的内存去哪了？](https://www.cnblogs.com/t-bar/p/17359545.html).
## 参考
+ [vmtouch](https://hoytech.com/vmtouch/)