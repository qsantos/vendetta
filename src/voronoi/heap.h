/*\
 *  Voronoi diagram by Fortune's algorithm
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

#ifndef HEAP_H
#define HEAP_H

typedef struct hnode hnode_t;
typedef struct heap  heap_t;

#include <sys/types.h>

struct hnode
{
	double idx;
	void* data;
};

struct heap
{
	size_t   size;
	size_t   avail;
	hnode_t* tree;
};

void heap_init(heap_t* h);
void heap_exit(heap_t* h);

void  heap_insert(heap_t* h, double idx, void* data);
void* heap_remove(heap_t* h);

#endif
