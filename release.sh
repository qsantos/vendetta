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

time make -j 4 -C src rebuild
OS="linux-64bits"
release="$name-$version-$OS"
archive=${release}.tgz
ln -s . $release
(git ls-files; ls start.sh lib/* vendetta) | sed "s/^/$release\//" | xargs -d $'\n' tar zcf $archive
rm $release
scp $archive micronos:servers/www/vendetta/dl/ &

time make -j 4 -C src rebuild CC=i586-mingw32msvc-gcc
mv vendetta vendetta.exe
OS="windows-32bits"
release="$name-$version-$OS"
archive=${release}.zip
ln -s . $release
mv win32/*.dll .
(git ls-files; ls *.dll vendetta.exe) | sed "s/^/$release\//" | xargs -d $'\n' zip -qr $archive
mv *.dll win32/
rm $release
mv vendetta.exe vendetta
scp $archive micronos:servers/www/vendetta/dl/ &

time make -j 4 -C src rebuild CC=x86_64-w64-mingw32-gcc
mv vendetta vendetta.exe
OS="windows-64bits"
release="$name-$version-$OS"
archive=${release}.zip
ln -s . $release
mv win64/*.dll .
(git ls-files; ls *.dll vendetta.exe) | sed "s/^/$release\//" | xargs -d $'\n' zip -qr $archive
mv *.dll win64/
rm $release
mv vendetta.exe vendetta
scp $archive micronos:servers/www/vendetta/dl/ &
