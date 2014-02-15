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

#include "effect.h"

#include <stdlib.h>

#include "../mem.h"

void effect_init(effect_t* e)
{
	e->n_skills = 0;
	e->skills   = NULL;
	e->bonuses  = NULL;

	for (size_t i = 0; i < N_STATUSES; i++)
		e->status_bonus[i] = 0;
}

void effect_exit(effect_t* e)
{
	free(e->bonuses);
	free(e->skills);
}

void effect_skill(effect_t* e, int skill, float bonus)
{
	e->skills  = CREALLOC(e->skills,  int,   e->n_skills+1);
	e->bonuses = CREALLOC(e->bonuses, float, e->n_skills+1);
	e->skills [e->n_skills] = skill;
	e->bonuses[e->n_skills] = bonus;
	e->n_skills++;
}
