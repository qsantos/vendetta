#!/bin/bash
if [ $# -lt 1 ]
then
	echo "Usage: $0 version"
	exit 1
fi

set -e

cd $(dirname $0)
name="vendetta"
version=$1

time make -j 4 -C src rebuild CC=gcc
OS="linux-64bits"
release="$name-$version-$OS"
archive=${release}.tgz
ln -s . $release
mv deps/lin64/*.so* .
(ls -d data vendetta *.so* start.sh) | sed "s/^/$release\//" | xargs -d $'\n' tar zcf $archive
mv *.so* deps/lin64/
rm $release

time make -j 4 -C src rebuild CC=i686-w64-mingw32-gcc
mv vendetta vendetta.exe
OS="windows-32bits"
release="$name-$version-$OS"
archive=${release}.zip
ln -s . $release
mv deps/win32/*.dll .
(ls -d data vendetta.exe *.dll debug.bat) | sed "s/^/$release\//" | xargs -d $'\n' zip -qr $archive
mv *.dll deps/win32/
rm $release
mv vendetta.exe vendetta

time make -j 4 -C src rebuild CC=x86_64-w64-mingw32-gcc
mv vendetta vendetta.exe
OS="windows-64bits"
release="$name-$version-$OS"
archive=${release}.zip
ln -s . $release
mv deps/win64/*.dll .
(ls -d data vendetta.exe *.dll debug.bat) | sed "s/^/$release\//" | xargs -d $'\n' zip -qr $archive
mv *.dll deps/win64/
rm $release
mv vendetta.exe vendetta
