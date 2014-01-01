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

#ifndef U_ITEM_H
#define U_ITEM_H

typedef struct kindOf_item kindOf_item_t;

#include <wchar.h>

#include "universe.h"
#include "skill.h"

struct kindOf_item
{
	wchar_t* name;

	int skill;
	int category;

	float  bonus_special[N_SPECIAL_SKILLS];
	float* bonus_material;
	float* bonus_item;
};

void kindOf_item_init(kindOf_item_t* i, universe_t* u);
void kindOf_item_exit(kindOf_item_t* i);

#endif
