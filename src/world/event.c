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

#include "event.h"

#include <sys/types.h>
#include <stdlib.h>
#include <string.h>

#include "../mem.h"

void evtList_init(evtList_t* l)
{
	l->n = 0;
	l->d = NULL;
}

void evtList_exit(evtList_t* l)
{
	free(l->d);
}

void evtList_push(evtList_t* l, kindOf_event_t* t, float x, float y)
{
	l->d = CREALLOC(l->d, event_t, l->n+1);
	l->d[l->n++] = (event_t){t, x, y, 0};

	if (t->sound != NULL)
		sfSound_play(t->sound);
}

void evtList_doRound(evtList_t* l, float duration)
{
	for (ssize_t i = 0; i < (ssize_t) l->n; i++)
	{
		event_t* e = &l->d[i];
		e->p += duration / e->t->duration;
		if (e->p >= 1)
		{
			l->n--;
			memmove(l->d+i, l->d+i+1, sizeof(event_t)*(l->n-i));
			i--;
		}
	}
}
