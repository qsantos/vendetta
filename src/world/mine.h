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

#ifndef W_MINE_H
#define W_MINE_H

typedef struct mine mine_t;

#include "object.h"
#include "world.h"
#include "../universe/mine.h"

struct mine
{
	object_t o;
	world_t* w;
	kindOf_mine_t* t;
};

void mine_init(mine_t* m, world_t* w, kindOf_mine_t* t, float x, float y);
void mine_exit(mine_t* m);

#endif
