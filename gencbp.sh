#!/bin/bash
cd $(dirname $0)
cat << EOF
<?xml version="1.0" encoding="UTF-8" standalone="yes" ?>
<CodeBlocks_project_file>
<FileVersion major="1" minor="6" />
<Project>

<Option title="vendetta" />
<Option makefile_is_custom="1" />
<Option execution_dir="." />
<Build>
	<Target title="all">
		<Option output="../vendetta" prefix_auto="1" extension_auto="1" />
		<Option working_dir=".." />
		<Option object_output="." />
		<Option type="0" />
	</Target>
</Build>
EOF

for file in $(git ls-files)
do
	echo "<Unit filename=\"$file\" />"
done

cat << EOF

</Project>
</CodeBlocks_project_file>
EOF
