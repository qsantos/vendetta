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

#ifndef U_MINE_H
#define U_MINE_H

typedef struct kindOf_mine kindOf_mine_t;

#include "transform.h"

struct kindOf_mine
{
	int id;
	char* name;

	transform_t harvest;
};

void kindOf_mine_init(kindOf_mine_t* m, int id);
void kindOf_mine_exit(kindOf_mine_t* m);

#endif
