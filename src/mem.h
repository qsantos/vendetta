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

#ifndef MEM_H
#define MEM_H

void* check_alloc(size_t n, void* ptr, const char* file, int line);

#define CALLOC(T,N)     ( (T*) check_alloc((N)*sizeof(T), NULL, __FILE__, __LINE__) )
#define CREALLOC(P,T,N) ( (T*) check_alloc((N)*sizeof(T), P,    __FILE__, __LINE__) )

#endif
