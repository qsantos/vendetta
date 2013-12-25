#include "subwindow.h"

void subwindow_init(subwindow_t* w)
{
	w->x = 0;
	w->y = 0;
}

void subwindow_exit(subwindow_t* w)
{
	(void) w;
}

void subwindow_draw(subwindow_t* w, graphics_t* g)
{
	sfSprite* sprite = NULL;
	if (sprite == NULL)
	{
		int id = graphics_spriteForImg(g, "subwindow.png");
		sprite = g->sprites[id];
	}
	sfVector2f pos = {w->x, w->y};
	sfSprite_setPosition(sprite, pos);
	sfRenderWindow_drawSprite(g->render, sprite, NULL);
}
