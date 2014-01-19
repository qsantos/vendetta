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

#ifndef U_EFFECT_H
#define U_EFFECT_H

typedef struct effect effect_t;

#include <sys/types.h>

struct effect
{
	size_t n_skills;
	int*   skills;
	float* bonuses;
};

void effect_init(effect_t* e);
void effect_exit(effect_t* e);

void effect_skill(effect_t* e, int skill, float bonus);

#endif
