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

#ifndef W_PROJECTILE_H
#define W_PROJECTILE_H

typedef struct projectile projectile_t;

#include "object.h"
#include "../universe/projectile.h"

struct projectile
{
	object_t o;

	kindOf_projectile_t* t;

	float target_x;
	float target_y;

	direction_t dir;
	float step;
};

void projectile_init(projectile_t* p, kindOf_projectile_t* t, float x, float y, float tx, float ty);
void projectile_exit(projectile_t* p);

void projectile_doRound(projectile_t* p, float duration);

#endif
