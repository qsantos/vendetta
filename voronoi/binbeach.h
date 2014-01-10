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

#ifndef BINBEACH_H
#define BINBEACH_H

typedef struct vr_bnode    vr_bnode_t;
typedef struct vr_binbeach vr_binbeach_t;

#include <sys/types.h>

#include "geometry.h"

struct vr_vertex;
struct vr_region;
struct vr_event;

// internal nodes are breakpoints
// (two regions, two children, 'end' set)
// leaves are arcs (one region, no child, 'event' set)
struct vr_bnode
{
	struct vr_region* r1;
	struct vr_region* r2;

	vr_bnode_t* left;
	vr_bnode_t* right;
	vr_bnode_t* parent;

	// linked point id
	struct vr_vertex** end;

	struct vr_event* event;
};

struct vr_binbeach
{
	vr_bnode_t* root;
};

void vr_binbeach_init(vr_binbeach_t* b);
void vr_binbeach_exit(vr_binbeach_t* b);

vr_bnode_t* vr_binbeach_breakAt(vr_binbeach_t* b, double sweep, struct vr_region* r);

// vr_bnode_X finds closest ancestor of n for which n is X to
vr_bnode_t* vr_bnode_left (vr_bnode_t* n);
vr_bnode_t* vr_bnode_right(vr_bnode_t* n);

vr_bnode_t* vr_bnode_prev(vr_bnode_t* n);
vr_bnode_t* vr_bnode_next(vr_bnode_t* n);

// remove an arc, return the new breakpoint
vr_bnode_t* vr_bnode_remove(vr_bnode_t* n);

#endif
