@echo off
echo Building tend...

cargo build
set TEND=.\target\debug\tend.exe

echo Running tests...

@echo on
%TEND% delete --all --confirm
%TEND% list
%TEND% create hello ping 1.1.1.1
%TEND% list
%TEND% create world ping -g group 8.8.8.8
%TEND% list
%TEND% create hello --restart=always ping 1.1.1.1
%TEND% create hello --overwrite --restart=always ping 1.1.1.1
%TEND% list
%TEND% delete hello
%TEND% list
%TEND% create hello ping 1.1.1.1
%TEND% list
%TEND% delete -j hello
%TEND% list
%TEND% create hello ping 1.1.1.1
%TEND% list
%TEND% delete -g default
%TEND% list
%TEND% create hello ping 1.1.1.1
%TEND% create hello2 ping -g group2 2.2.2.2
%TEND% create hello222 ping -g group2 222.222.222.222
%TEND% list
%TEND% list hello
%TEND% list world
%TEND% list -g group
%TEND% list -g default
%TEND% list -g default -j hello
%TEND% list -g group2 -j hello
%TEND% list --all
%TEND% list --all -e hello
%TEND% list --all -e hello world
%TEND% list --all -e hello world hello2
%TEND% list --all -e hello world hello2 hello222
%TEND% list --all -g group2
%TEND% list --all -j hello
%TEND% list -g group2 --all
%TEND% list -j hello --all
%TEND% delete --all -g group2
%TEND% delete --all -j hello
%TEND% delete -g group2 --all
%TEND% delete -j hello --all
%TEND% delete -g group2 --all
%TEND% list
%TEND% delete -g group2
%TEND% list
%TEND% create hello2 ping -g group2 2.2.2.2
%TEND% create hello222 ping -g group2 222.222.222.222
%TEND% delete -g group2 -e hello2
%TEND% list
%TEND% delete -g group2
%TEND% list
%TEND% delete -j hello world
%TEND% list

echo Done.
