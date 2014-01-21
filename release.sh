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
release="$name-$version-linux-64bits"
archive=${release}.tgz
ln -s . $release
(git ls-files; ls start.sh lib/* vendetta) | sed "s/^/$release\//" | xargs -d $'\n' tar zcf $archive
rm $release
scp $archive micronos:servers/www/vendetta/dl/ &

time make -j 4 -C src rebuild CC=i586-mingw32msvc-gcc
mv vendetta vendetta.exe
release="$name-$version-windows-32bits"
archive=${release}.zip
ln -s . $release
(git ls-files; ls *.dll vendetta.exe) | sed "s/^/$release\//" | xargs -d $'\n' zip -qr $archive
rm $release
mv vendetta.exe vendetta
scp $archive micronos:servers/www/vendetta/dl/
