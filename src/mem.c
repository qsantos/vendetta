/*\
 *  Role playing, management and strategy game
 *  Copyright (C) 2013-2014 Quentin SANTOS
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <http://www.gnu.org/licenses/>.
\*/

#ifndef MEM_C
#define MEM_C

#include <stdlib.h>
#include <stdio.h>

void* check_alloc(size_t n, void* ptr, const char* file, int line)
{
	void* ret = realloc(ptr, n);
	if (ret == NULL)
	{
		fprintf(stderr, "Could not allocate memory at '%s' line %i\n", file, line);
		exit(1);
	}
	return ret;
}

#endif
