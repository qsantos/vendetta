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

#include "projectile.h"

void projectile_init(projectile_t* p, kindOf_projectile_t* t, float x, float y, float tx, float ty)
{
	p->o.t = O_PROJECTILE;
	p->o.x = x;
	p->o.y = y;
	p->o.w = t->width;
	p->o.h = t->height;
	p->t = t;
	p->target_x = tx;
	p->target_y = ty;
	p->dir = D_SOUTH;
	p->step = 0;
}

void projectile_exit(projectile_t* p)
{
	(void) p;
}
